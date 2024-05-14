//! A helper macro for defining "SomethingIdentifier" enums.
//!
//! These all share largely the same code, so this macro reduces code duplication.

/// Define an "identifier" type.
macro_rules! identifier {
	(
		$(#[$docs:meta])*
		enum $name:ident {
			$(
			$(#[$variant_docs:meta])*
			$variant:ident($ty:ident)
			),*
			$(,)?
		}

		ParseError: $parse_error:ident
	) => {
		$(#[$docs])*
		#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
		#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
		pub enum $name {
			$( $(#[$variant_docs])* $variant($ty) ),*
		}

		impl std::fmt::Display for $name {
			fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
				match *self {
					$( Self::$variant(ref v) => std::fmt::Display::fmt(v, f) ),*
				}
			}
		}

		$(impl From<$ty> for $name {
			fn from(value: $ty) -> Self {
				Self::$variant(value)
			}
		})*

		#[derive(Debug, Clone, thiserror::Error)]
		#[error("unrecognized format `{0}`")]
		pub struct $parse_error(pub String);

		impl std::str::FromStr for $name {
			type Err = $parse_error;

			fn from_str(s: &str) -> Result<Self, Self::Err> {
				$(if let Ok(v) = <$ty as std::str::FromStr>::from_str(s) {
					return Ok(Self::from(v));
				})*

				Err($parse_error(s.to_owned()))
			}
		}

		/// Method and Trait implementations when depending on [`serde`].
		#[cfg(feature = "serde")]
		mod serde_impls {
			use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

			use super::*;

			impl Serialize for $name {
				fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
				where
					S: Serializer,
				{
					match *self {
						$( Self::$variant(ref v) => Serialize::serialize(v, serializer) ),*
					}
				}
			}

			impl<'de> Deserialize<'de> for $name {
				fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
				where
					D: Deserializer<'de>,
				{
					#[derive(Deserialize)]
					#[serde(untagged)]
					#[allow(clippy::missing_docs_in_private_items, non_camel_case_types)]
					enum Helper {
						$( $ty($ty) ),*
					}

					#[allow(unreachable_patterns)]
					Helper::deserialize(deserializer).and_then(|value| match value {
						Helper::String(str) => str.parse().map_err(de::Error::custom),
						$( Helper::$ty(v) => Ok(Self::from(v)) ),*
					})
				}
			}
		}
	};
}

pub(crate) use identifier;
