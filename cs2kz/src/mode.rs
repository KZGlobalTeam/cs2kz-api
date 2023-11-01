use {
	crate::{Error, Result},
	std::{fmt::Display, str::FromStr},
};

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub enum Mode {
	#[default]
	#[cfg_attr(feature = "utoipa", schema(rename = "kz_modded"))]
	Modded = 1,
	#[cfg_attr(feature = "utoipa", schema(rename = "kz_vanilla"))]
	Vanilla = 2,
}

impl Mode {
	pub const fn api(&self) -> &'static str {
		match self {
			Mode::Modded => "kz_modded",
			Mode::Vanilla => "kz_vanilla",
		}
	}
}

impl Display for Mode {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{self:?}")
	}
}

macro_rules! try_from {
	([$($t:ty),+]) => {
		$(impl TryFrom<$t> for Mode {
			type Error = $crate::Error;

			fn try_from(value: $t) -> $crate::Result<Self> {
				match value {
					1 => Ok(Self::Modded),
					2 => Ok(Self::Vanilla),
					_ => Err($crate::Error::InvalidMode {
						input: value.to_string(),
						reason: Some(String::from("invalid ID")),
					}),
				}
			}
		})+
	};
}

try_from!([u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize]);

impl TryFrom<&str> for Mode {
	type Error = Error;

	fn try_from(input: &str) -> Result<Self> {
		input.parse()
	}
}

impl TryFrom<String> for Mode {
	type Error = Error;

	fn try_from(input: String) -> Result<Self> {
		Self::try_from(input.as_str())
	}
}

impl FromStr for Mode {
	type Err = Error;

	fn from_str(input: &str) -> Result<Self> {
		match input {
			"kz_modded" | "modded" | "mod" => Ok(Self::Modded),
			"kz_vanilla" | "vanilla" | "vnl" => Ok(Self::Vanilla),
			_ => Err(Error::InvalidMode { input: input.to_owned(), reason: None }),
		}
	}
}

#[cfg(feature = "serde")]
mod serde_impls {
	use {
		super::Mode,
		serde::{Deserialize, Deserializer, Serialize, Serializer},
	};

	impl Serialize for Mode {
		fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
		where
			S: Serializer, {
			self.api().serialize(serializer)
		}
	}

	impl<'de> Deserialize<'de> for Mode {
		fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
		where
			D: Deserializer<'de>, {
			String::deserialize(deserializer)?
				.parse()
				.map_err(serde::de::Error::custom)
		}
	}
}
