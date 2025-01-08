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

//! Helper functions & types for using Steam as an OpenID 2.0 provider.

#![feature(substr_range)]

#[macro_use]
extern crate derive_more;

pub const LOGIN_URL: &str = "https://steamcommunity.com/openid/login";

mod login_url;
pub use login_url::login_url;

mod callback_payload;
pub use callback_payload::{CallbackPayload, VerifyCallbackPayloadError};
