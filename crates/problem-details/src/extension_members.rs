use std::borrow::Borrow;
use std::fmt;
use std::hash::Hash;

use serde::Serialize;
use serde_json::map as base;

type Fields = serde_json::Map<String, serde_json::Value>;

/// Extra fields to include in [`ProblemDetails`].
///
/// This corresponds to [Section 3.2] of the [RFC].
///
/// [Section 3.2]: https://www.rfc-editor.org/rfc/rfc9457.html#section-3.2
/// [RFC]: https://www.rfc-editor.org/rfc/rfc9457.html
#[derive(Default, Clone, PartialEq, Eq)]
pub struct ExtensionMembers {
    fields: Fields,
}

pub struct Iter<'a> {
    fields: base::Iter<'a>,
}

pub struct IterMut<'a> {
    fields: base::IterMut<'a>,
}

pub struct IntoIter {
    fields: base::IntoIter,
}

impl ExtensionMembers {
    /// Creates a new empty [`ExtensionMembers`].
    pub fn new() -> Self {
        Self::default()
    }

    pub fn count(&self) -> usize {
        self.fields.len()
    }

    pub fn get<Q>(&self, key: &Q) -> Option<&serde_json::Value>
    where
        String: Borrow<Q>,
        Q: Eq + Ord + Hash + ?Sized,
    {
        self.fields.get(key)
    }

    pub fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut serde_json::Value>
    where
        String: Borrow<Q>,
        Q: Eq + Ord + Hash + ?Sized,
    {
        self.fields.get_mut(key)
    }

    /// Adds a new extension member.
    ///
    /// If there was already a member for the given `key`, its old value will be returned.
    pub fn add<V>(
        &mut self,
        key: impl Into<String>,
        value: &V,
    ) -> Result<Option<serde_json::Value>, serde_json::Error>
    where
        V: Serialize + ?Sized,
    {
        serde_json::to_value(value).map(|value| self.fields.insert(key.into(), value))
    }

    pub fn iter(&self) -> Iter<'_> {
        self.into_iter()
    }

    pub fn iter_mut(&mut self) -> IterMut<'_> {
        self.into_iter()
    }
}

impl fmt::Debug for ExtensionMembers {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_map().entries(&self.fields).finish()
    }
}

impl<'a> IntoIterator for &'a ExtensionMembers {
    type Item = (&'a str, &'a serde_json::Value);
    type IntoIter = Iter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        Iter { fields: self.fields.iter() }
    }
}

impl<'a> IntoIterator for &'a mut ExtensionMembers {
    type Item = (&'a str, &'a mut serde_json::Value);
    type IntoIter = IterMut<'a>;

    fn into_iter(self) -> Self::IntoIter {
        IterMut { fields: self.fields.iter_mut() }
    }
}

impl IntoIterator for ExtensionMembers {
    type Item = (String, serde_json::Value);
    type IntoIter = IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter { fields: self.fields.into_iter() }
    }
}

macro_rules! impl_iterator {
    ($iter:ident $( < $($lt:lifetime),* $(,)? > )? yields $item:ty => |$key:ident| $map:expr) => {
        impl $(< $($lt),* >)? Iterator for $iter $(< $($lt),* >)? {
            type Item = $item;

            fn next(&mut self) -> Option<Self::Item> {
                self.fields.next().map(|($key, v)| ($map, v))
            }

            fn size_hint(&self) -> (usize, Option<usize>) {
                self.fields.size_hint()
            }
        }

        impl $(< $($lt),* >)? DoubleEndedIterator for $iter $(< $($lt),* >)? {
            fn next_back(&mut self) -> Option<Self::Item> {
                self.fields.next_back().map(|($key, v)| ($map, v))
            }
        }

        impl $(< $($lt),* >)? ExactSizeIterator for $iter $(< $($lt),* >)? {
            fn len(&self) -> usize {
                self.fields.len()
            }
        }

        impl $(< $($lt),* >)? std::iter::FusedIterator for $iter $(< $($lt),* >)? {}
    };
}

impl_iterator!(Iter<'a> yields (&'a str, &'a serde_json::Value) => |key| key.as_str());
impl_iterator!(IterMut<'a> yields (&'a str, &'a mut serde_json::Value) => |key| key.as_str());
impl_iterator!(IntoIter yields (String, serde_json::Value) => |key| key);
