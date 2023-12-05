use std::fmt::{self, Display};
use std::str::FromStr;

use crate::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub enum Jumpstat {
	#[cfg_attr(feature = "utoipa", schema(rename = "longjump"))]
	LongJump = 1,

	#[cfg_attr(feature = "utoipa", schema(rename = "single_bhop"))]
	SingleBhop = 2,

	#[cfg_attr(feature = "utoipa", schema(rename = "multi_bhop"))]
	MultiBhop = 3,

	#[cfg_attr(feature = "utoipa", schema(rename = "drop_bhop"))]
	DropBhop = 4,

	#[cfg_attr(feature = "utoipa", schema(rename = "weirdjump"))]
	WeirdJump = 5,

	#[cfg_attr(feature = "utoipa", schema(rename = "ladderjump"))]
	LadderJump = 6,

	#[cfg_attr(feature = "utoipa", schema(rename = "ladderhop"))]
	LadderHop = 7,
}

impl Jumpstat {
	pub const fn api(&self) -> &'static str {
		match self {
			Jumpstat::LongJump => "longjump",
			Jumpstat::SingleBhop => "single_bhop",
			Jumpstat::MultiBhop => "multi_bhop",
			Jumpstat::DropBhop => "drop_bhop",
			Jumpstat::WeirdJump => "weirdjump",
			Jumpstat::LadderJump => "ladderjump",
			Jumpstat::LadderHop => "ladderhop",
		}
	}
}

impl Display for Jumpstat {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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
					2 => Ok(Self::SingleBhop),
					3 => Ok(Self::MultiBhop),
					4 => Ok(Self::DropBhop),
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

#[rustfmt::skip]
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
			"bhop" | "single_bhop" => Ok(Self::SingleBhop),
			"multi_bhop" => Ok(Self::MultiBhop),
			"drop_bhop" => Ok(Self::DropBhop),
			"weirdjump" => Ok(Self::WeirdJump),
			"ladderjump" => Ok(Self::LadderJump),
			"ladderhop" => Ok(Self::LadderHop),
			_ => Err(Error::InvalidJumpstat { input: input.to_owned(), reason: None }),
		}
	}
}

#[cfg(feature = "serde")]
mod serde_impls {
	use serde::{Deserialize, Deserializer, Serialize, Serializer};

	use super::Jumpstat;

	impl Serialize for Jumpstat {
		fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
		where
			S: Serializer,
		{
			self.api().serialize(serializer)
		}
	}

	impl<'de> Deserialize<'de> for Jumpstat {
		fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
		where
			D: Deserializer<'de>,
		{
			String::deserialize(deserializer)?
				.parse()
				.map_err(serde::de::Error::custom)
		}
	}
}
