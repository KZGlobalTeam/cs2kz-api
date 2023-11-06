use {
	crate::{Error, Result},
	std::{fmt::Display, str::FromStr},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "utoipa", schema(rename_all = "snake_case"))]
pub enum Tier {
	VeryEasy = 1,
	Easy = 2,
	Medium = 3,
	Hard = 4,
	VeryHard = 5,
	Extreme = 6,
	Tier7 = 7,
	Tier8 = 8,
	Death = 9,
	Impossible = 10,
}

impl Tier {
	pub const fn api(&self) -> &'static str {
		match self {
			Tier::VeryEasy => "very_easy",
			Tier::Easy => "easy",
			Tier::Medium => "medium",
			Tier::Hard => "hard",
			Tier::VeryHard => "very_hard",
			Tier::Extreme => "extreme",
			Tier::Tier7 => "tier7",
			Tier::Tier8 => "tier8",
			Tier::Death => "death",
			Tier::Impossible => "impossible",
		}
	}
}

impl Display for Tier {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{self:?}")
	}
}

macro_rules! try_from {
	([$($t:ty),+]) => {
		$(impl TryFrom<$t> for Tier {
			type Error = $crate::Error;

			fn try_from(value: $t) -> $crate::Result<Self> {
				match value {
					1 => Ok(Self::VeryEasy),
					2 => Ok(Self::Easy),
					3 => Ok(Self::Medium),
					4 => Ok(Self::Hard),
					5 => Ok(Self::VeryHard),
					6 => Ok(Self::Extreme),
					7 => Ok(Self::Tier7),
					8 => Ok(Self::Tier8),
					9 => Ok(Self::Death),
					10 => Ok(Self::Impossible),
					_ => Err($crate::Error::InvalidTier {
						input: value.to_string(),
						reason: Some(String::from("invalid ID")),
					}),
				}
			}
		})+
	};
}

try_from!([u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize]);

impl TryFrom<&str> for Tier {
	type Error = Error;

	fn try_from(input: &str) -> Result<Self> {
		input.parse()
	}
}

impl TryFrom<String> for Tier {
	type Error = Error;

	fn try_from(input: String) -> Result<Self> {
		Self::try_from(input.as_str())
	}
}

impl FromStr for Tier {
	type Err = Error;

	fn from_str(input: &str) -> Result<Self> {
		match input {
			"very_easy" => Ok(Tier::VeryEasy),
			"easy" => Ok(Tier::Easy),
			"medium" => Ok(Tier::Medium),
			"hard" => Ok(Tier::Hard),
			"very_hard" => Ok(Tier::VeryHard),
			"extreme" => Ok(Tier::Extreme),
			"tier7" => Ok(Tier::Tier7),
			"tier8" => Ok(Tier::Tier8),
			"death" => Ok(Tier::Death),
			"impossible" => Ok(Tier::Impossible),
			_ => Err(Error::InvalidTier { input: input.to_owned(), reason: None }),
		}
	}
}

#[cfg(feature = "serde")]
mod serde_impls {
	use {
		super::Tier,
		serde::{Deserialize, Deserializer, Serialize, Serializer},
	};

	impl Serialize for Tier {
		fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
		where
			S: Serializer, {
			self.api().serialize(serializer)
		}
	}

	impl<'de> Deserialize<'de> for Tier {
		fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
		where
			D: Deserializer<'de>, {
			String::deserialize(deserializer)?
				.parse()
				.map_err(serde::de::Error::custom)
		}
	}
}
