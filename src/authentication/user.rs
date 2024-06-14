//! Authenticated users.
//!
//! These are KZ players and admins authenticated via [sessions], e.g. on a website.
//!
//! [sessions]: super::Session

use cs2kz::SteamID;
use derive_more::Debug;

use crate::authorization::Permissions;

/// An authenticated user.
#[derive(Debug, Clone, Copy)]
pub struct User {
	/// The user's [SteamID].
	#[debug("{steam_id}")]
	steam_id: SteamID,

	/// The user's permissions.
	#[debug("{permissions} ({permissions:?})")]
	permissions: Permissions,
}

impl User {
	/// Creates a new [`User`].
	pub const fn new(steam_id: SteamID, permissions: Permissions) -> Self {
		Self {
			steam_id,
			permissions,
		}
	}

	/// Returns this user's [SteamID].
	pub const fn steam_id(&self) -> SteamID {
		self.steam_id
	}

	/// Returns this user's permissions.
	pub const fn permissions(&self) -> Permissions {
		self.permissions
	}
}
