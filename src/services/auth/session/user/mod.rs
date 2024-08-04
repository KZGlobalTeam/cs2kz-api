//! This module contains the [`User`] type, which represents a user that is
//! associated with a [`Session`]
//!
//! [`Session`]: super::Session

use cs2kz::SteamID;

pub mod permissions;
pub use permissions::Permissions;

/// An authenticated user.
#[derive(Debug, Clone)]
pub struct User
{
	/// The user's SteamID.
	steam_id: SteamID,

	/// The user's permissions.
	permissions: Permissions,
}

impl User
{
	/// Creates a new [`User`].
	pub fn new(steam_id: SteamID, permissions: Permissions) -> Self
	{
		Self { steam_id, permissions }
	}

	/// Returns this user's [SteamID].
	pub fn steam_id(&self) -> SteamID
	{
		self.steam_id
	}

	/// Returns this user's permissions.
	pub fn permissions(&self) -> Permissions
	{
		self.permissions
	}
}
