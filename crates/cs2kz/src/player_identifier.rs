//! Different ways of identifying players.

use crate::SteamID;

crate::identifier::identifier! {
	/// Different ways of identifying a player.
	enum PlayerIdentifier {
		/// A [SteamID].
		SteamID(SteamID),

		/// A player name.
		Name(String),
	}

	ParseError: ParsePlayerIdentifierError
}

/// Method and Trait implementations when depending on [`utoipa`].
#[cfg(feature = "utoipa")]
mod utoipa_impls {
	use utoipa::openapi::path::{Parameter, ParameterBuilder, ParameterIn};
	use utoipa::{IntoParams, ToSchema};

	use crate::PlayerIdentifier;

	impl IntoParams for PlayerIdentifier {
		fn into_params(parameter_in_provider: impl Fn() -> Option<ParameterIn>) -> Vec<Parameter> {
			vec![
				ParameterBuilder::new()
					.name("player")
					.parameter_in(parameter_in_provider().unwrap_or_default())
					.schema(Some(Self::schema().1))
					.build(),
			]
		}
	}
}
