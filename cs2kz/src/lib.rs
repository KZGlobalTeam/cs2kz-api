// Copyright (C) AlphaKeks <alphakeks@dawn.sh>
//
// This is free software. You can redistribute it and / or modify it under the terms of the
// GNU General Public License as published by the Free Software Foundation, either version 3
// of the License, or (at your option) any later version.
//
// You should have received a copy of the GNU General Public License along with this repository.
// If not, see <https://www.gnu.org/licenses/>.

//! CS2KZ

mod error;
pub use error::{Error, Result};

pub mod steam_id;
pub use steam_id::SteamID;

pub mod player_identifier;
pub use player_identifier::PlayerIdentifier;

pub mod mode;
pub use mode::Mode;

pub mod style;
pub use style::Style;

#[cfg(test)]
mod test_setup {
	#[ctor::ctor]
	fn test_setup() {
		color_eyre::install().expect("Failed to setup color-eyre.");
	}
}
