/* Copyright (C) 2024  AlphaKeks <alphakeks@dawn.sh>
 *
 * This library is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This library is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this repository.  If not, see <https://www.gnu.org/licenses/>.
 */

//! # [RFC 9457][rfc] - Problem Details for HTTP APIs
//!
//! This crate provides an implementation of [RFC 9457][rfc] that can be used with the [`http`]
//! crate and compatible frameworks.
//!
//! [rfc]: https://www.rfc-editor.org/rfc/rfc9457.html

#![feature(debug_closure_helpers)]

use std::any::type_name;
use std::borrow::Cow;
use std::fmt;

use mime::Mime;
use serde::ser::{Serialize, SerializeMap, Serializer};

pub mod extension_members;
pub use extension_members::ExtensionMembers;

/// Returns the [`Content-Type`] value used in responses.
///
/// [`Content-Type`]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Content-Type
pub fn content_type() -> Mime {
    "application/problem+json"
        .parse::<Mime>()
        .expect("hard-coded string should always be valid")
}

/// An [RFC 9457][rfc] compliant response.
///
/// This object can be serialized into JSON via [`serde`].
///
/// [rfc]: https://www.rfc-editor.org/rfc/rfc9457.html
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ProblemDetails<T: ?Sized> {
    /// The response's ["detail"] member.
    ///
    /// ["detail"]: https://www.rfc-editor.org/rfc/rfc9457.html#section-3.1.4
    detail: Option<Cow<'static, str>>,

    /// The response's ["instance"] member.
    ///
    /// ["instance"]: https://www.rfc-editor.org/rfc/rfc9457.html#section-3.1.5
    instance: Option<Cow<'static, str>>,

    /// Additional fields to include in the response.
    ///
    /// This corresponds to [Section 3.2] of the [RFC].
    ///
    /// [Section 3.2]: https://www.rfc-editor.org/rfc/rfc9457.html#section-3.2
    /// [RFC]: https://www.rfc-editor.org/rfc/rfc9457.html
    extension_members: ExtensionMembers,

    /// The problem type.
    ///
    /// This corresponds to the ["type"] member in the response. This is generic so downstream
    /// users can choose their own problem types. The type you choose here should implement the
    /// [`ProblemType`] trait.
    ///
    /// ["type"]: https://www.rfc-editor.org/rfc/rfc9457.html#section-3.1.1
    // this is the last field so `ProblemDetails` can be coerced into a DST
    problem_type: T,
}

/// A problem type.
///
/// This can be used to customize the behavior of <code>[ProblemDetails]\<T></code> where <code>T:
/// [ProblemType]</code>.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not an HTTP problem type",
    note = "only types that implement `ProblemType` can be used with `ProblemDetails<T>`"
)]
pub trait ProblemType {
    /// The URI to encode in the response's ["type"] member.
    ///
    /// ["type"]: https://www.rfc-editor.org/rfc/rfc9457.html#section-3.1.1
    fn uri(&self) -> http::Uri;

    /// The status code to use in the response.
    ///
    /// This is also the status code used in the response's ["status"] member.
    ///
    /// ["status"]: https://www.rfc-editor.org/rfc/rfc9457.html#section-3.1.2
    fn status(&self) -> http::StatusCode;

    /// The response's ["title"] member.
    ///
    /// ["title"]: https://www.rfc-editor.org/rfc/rfc9457.html#section-3.1.3
    fn title(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result;
}

impl<T> ProblemDetails<T> {
    /// Creates a new [`ProblemDetails`] object for the given [`ProblemType`].
    pub fn new(problem_type: T) -> Self {
        Self {
            problem_type,
            detail: None,
            instance: None,
            extension_members: ExtensionMembers::new(),
        }
    }
}

impl<T: ?Sized> ProblemDetails<T> {
    pub fn problem_type(&self) -> &T {
        &self.problem_type
    }

    pub fn problem_type_mut(&mut self) -> &mut T {
        &mut self.problem_type
    }

    pub fn detail(&self) -> Option<&str> {
        self.detail.as_deref()
    }

    pub fn instance(&self) -> Option<&str> {
        self.instance.as_deref()
    }

    pub fn extension_members(&self) -> &ExtensionMembers {
        &self.extension_members
    }

    pub fn extension_members_mut(&mut self) -> &mut ExtensionMembers {
        &mut self.extension_members
    }

    /// Populates the ["detail"] field.
    ///
    /// ["detail"]: https://www.rfc-editor.org/rfc/rfc9457.html#section-3.1.4
    pub fn set_detail(&mut self, detail: impl Into<Cow<'static, str>>) {
        self.detail = Some(detail.into());
    }

    /// Populates the ["instance"] field.
    ///
    /// ["instance"]: https://www.rfc-editor.org/rfc/rfc9457.html#section-3.1.5
    pub fn set_instance(&mut self, instance: impl Into<Cow<'static, str>>) {
        self.instance = Some(instance.into());
    }

    /// Adds an [extension member] field.
    ///
    /// # Panics
    ///
    /// This function will panic if `value` cannot be serialized into a JSON value.
    ///
    /// [extension member]: ExtensionMembers
    #[track_caller]
    pub fn add_extension<V>(&mut self, key: impl Into<String>, value: &V)
    where
        V: Serialize + ?Sized,
    {
        if let Err(error) = self.extension_members.add(key, value) {
            panic!("failed to serialize extension member of type `{}`: {error}", type_name::<V>());
        }
    }
}

impl<T: ProblemType + ?Sized> Serialize for ProblemDetails<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let field_count = 3 // type + status + title
            + usize::from(self.detail().is_some())
            + usize::from(self.instance().is_some())
            + self.extension_members().count();

        let mut serializer = serializer.serialize_map(Some(field_count))?;

        serializer.serialize_entry("type", &format_args!("{}", self.problem_type().uri()))?;
        serializer.serialize_entry("status", &self.problem_type().status().as_u16())?;

        #[rustfmt::skip]
        serializer.serialize_entry("title", &format_args!("{}", fmt::from_fn(|fmt| {
            self.problem_type().title(fmt)
        })))?;

        if let Some(detail) = self.detail() {
            serializer.serialize_entry("detail", detail)?;
        }

        if let Some(instance) = self.instance() {
            serializer.serialize_entry("instance", instance)?;
        }

        for (key, value) in self.extension_members() {
            serializer.serialize_entry(key, value)?;
        }

        serializer.end()
    }
}

impl<T: ProblemType> From<T> for ProblemDetails<T> {
    fn from(problem_type: T) -> Self {
        Self::new(problem_type)
    }
}

impl<T: ProblemType, B> From<ProblemDetails<T>> for http::Response<B>
where
    Vec<u8>: Into<B>,
{
    fn from(problem_details: ProblemDetails<T>) -> Self {
        (&problem_details).into()
    }
}

impl<T: ProblemType + ?Sized, B> From<&ProblemDetails<T>> for http::Response<B>
where
    Vec<u8>: Into<B>,
{
    fn from(problem_details: &ProblemDetails<T>) -> Self {
        let body = serde_json::to_vec(&problem_details)
            .expect("`ProblemDetails` should always serialize to JSON");

        http::Response::builder()
            .status(problem_details.problem_type().status())
            .header(http::header::CONTENT_TYPE, content_type().as_ref())
            .body(body.into())
            .expect("hard-coded response should be correct")
    }
}
