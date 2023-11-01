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

pub mod jumpstat;
pub use jumpstat::Jumpstat;

#[cfg(test)]
mod test_setup {
	#[ctor::ctor]
	fn test_setup() {
		color_eyre::install().expect("Failed to setup color-eyre.");
	}
}
