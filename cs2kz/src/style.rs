// Copyright (C) AlphaKeks <alphakeks@dawn.sh>
//
// This is free software. You can redistribute it and / or modify it under the terms of the
// GNU General Public License as published by the Free Software Foundation, either version 3
// of the License, or (at your option) any later version.
//
// You should have received a copy of the GNU General Public License along with this repository.
// If not, see <https://www.gnu.org/licenses/>.

use {
	crate::{Error, Result},
	std::{fmt::Display, str::FromStr},
	utoipa::ToSchema,
};

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, ToSchema)]
#[schema(rename_all = "snake_case")]
pub enum Style {
	#[default]
	Normal = 1,
	Backwards = 2,
	Sideways = 3,
	WOnly = 4,
}

impl Style {
	pub const fn api(&self) -> &'static str {
		match self {
			Style::Normal => "normal",
			Style::Backwards => "backwards",
			Style::Sideways => "sideways",
			Style::WOnly => "w_only",
		}
	}
}

impl Display for Style {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{self:?}")
	}
}

macro_rules! try_from {
	([$($t:ty),+]) => {
		$(impl TryFrom<$t> for Style {
			type Error = $crate::Error;

			fn try_from(value: $t) -> $crate::Result<Self> {
				match value {
					1 => Ok(Self::Normal),
					2 => Ok(Self::Backwards),
					3 => Ok(Self::Sideways),
					4 => Ok(Self::WOnly),
					_ => Err($crate::Error::InvalidStyle {
						input: value.to_string(),
						reason: Some(String::from("invalid ID")),
					}),
				}
			}
		})+
	};
}

try_from!([u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize]);

impl TryFrom<&str> for Style {
	type Error = Error;

	fn try_from(input: &str) -> Result<Self> {
		input.parse()
	}
}

impl TryFrom<String> for Style {
	type Error = Error;

	fn try_from(input: String) -> Result<Self> {
		Self::try_from(input.as_str())
	}
}

impl FromStr for Style {
	type Err = Error;

	fn from_str(input: &str) -> Result<Self> {
		match input {
			"normal" => Ok(Self::Normal),
			"backwards" => Ok(Self::Backwards),
			"sideways" => Ok(Self::Sideways),
			"w-only" | "w_only" => Ok(Self::WOnly),
			_ => Err(Error::InvalidStyle { input: input.to_owned(), reason: None }),
		}
	}
}

mod serde_impls {
	use {
		super::Style,
		serde::{Deserialize, Deserializer, Serialize, Serializer},
	};

	impl Serialize for Style {
		fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
		where
			S: Serializer, {
			self.api().serialize(serializer)
		}
	}

	impl<'de> Deserialize<'de> for Style {
		fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
		where
			D: Deserializer<'de>, {
			String::deserialize(deserializer)?
				.parse()
				.map_err(serde::de::Error::custom)
		}
	}
}
