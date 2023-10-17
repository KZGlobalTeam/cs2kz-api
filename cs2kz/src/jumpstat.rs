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
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub enum Jumpstat {
	#[cfg_attr(feature = "utoipa", schema(rename = "longjump"))]
	LongJump = 1,
	#[cfg_attr(feature = "utoipa", schema(rename = "bhop"))]
	BunnyHop = 2,
	#[cfg_attr(feature = "utoipa", schema(rename = "multi_bhop"))]
	MultiBunnyHop = 3,
	#[cfg_attr(feature = "utoipa", schema(rename = "drop_bhop"))]
	DropBunnyHop = 4,
	#[cfg_attr(feature = "utoipa", schema(rename = "weird_jump"))]
	WeirdJump = 5,
	#[cfg_attr(feature = "utoipa", schema(rename = "ladder_jump"))]
	LadderJump = 6,
	#[cfg_attr(feature = "utoipa", schema(rename = "ladder_hop"))]
	LadderHop = 7,
}

impl Jumpstat {
	pub const fn api(&self) -> &'static str {
		match self {
			Jumpstat::LongJump => "longjump",
			Jumpstat::BunnyHop => "bhop",
			Jumpstat::MultiBunnyHop => "multi_bhop",
			Jumpstat::DropBunnyHop => "drop_bhop",
			Jumpstat::WeirdJump => "weird_jump",
			Jumpstat::LadderJump => "ladder_jump",
			Jumpstat::LadderHop => "ladder_hop",
		}
	}
}

impl Display for Jumpstat {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{self:?}")
	}
}

macro_rules! try_from {
	([$($t:ty),+]) => {
		$(impl TryFrom<$t> for Jumpstat {
			type Error = $crate::Error;

			fn try_from(value: $t) -> $crate::Result<Self> {
				match value {
					1 => Ok(Self::LongJump),
					2 => Ok(Self::BunnyHop),
					3 => Ok(Self::MultiBunnyHop),
					4 => Ok(Self::DropBunnyHop),
					5 => Ok(Self::WeirdJump),
					6 => Ok(Self::LadderJump),
					7 => Ok(Self::LadderHop),
					_ => Err($crate::Error::InvalidJumpstat {
						input: value.to_string(),
						reason: Some(String::from("invalid ID")),
					}),
				}
			}
		})+
	};
}

try_from!([u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize]);

impl TryFrom<&str> for Jumpstat {
	type Error = Error;

	fn try_from(input: &str) -> Result<Self> {
		input.parse()
	}
}

impl TryFrom<String> for Jumpstat {
	type Error = Error;

	fn try_from(input: String) -> Result<Self> {
		Self::try_from(input.as_str())
	}
}

impl FromStr for Jumpstat {
	type Err = Error;

	fn from_str(input: &str) -> Result<Self> {
		match input {
			"longjump" => Ok(Self::LongJump),
			"bhop" => Ok(Self::BunnyHop),
			"multi_bhop" => Ok(Self::MultiBunnyHop),
			"drop_bhop" => Ok(Self::DropBunnyHop),
			"weird_jump" => Ok(Self::WeirdJump),
			"ladder_jump" => Ok(Self::LadderJump),
			"ladder_hop" => Ok(Self::LadderHop),
			_ => Err(Error::InvalidJumpstat { input: input.to_owned(), reason: None }),
		}
	}
}

#[cfg(feature = "serde")]
mod serde_impls {
	use {
		super::Jumpstat,
		serde::{Deserialize, Deserializer, Serialize, Serializer},
	};

	impl Serialize for Jumpstat {
		fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
		where
			S: Serializer, {
			self.api().serialize(serializer)
		}
	}

	impl<'de> Deserialize<'de> for Jumpstat {
		fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
		where
			D: Deserializer<'de>, {
			String::deserialize(deserializer)?
				.parse()
				.map_err(serde::de::Error::custom)
		}
	}
}
