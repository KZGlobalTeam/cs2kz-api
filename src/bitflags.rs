//! Helper macro for creating "bitflag" types.

use thiserror::Error;

/// Creates an integer wrapper that can be used for "flags".
macro_rules! bitflags {
	(
		$(#[$outer_meta:meta])*
		$vis:vis $name:ident as $repr:ty {
			$(
				$(#[$variant_meta:meta])*
				$variant:ident = { $value:expr, $variant_name:literal };
			)*
		}

		iter: $iter_name:ident
	) => {
		$(#[$outer_meta])*
		#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, ::sqlx::Type)]
		#[sqlx(transparent)]
		$vis struct $name($repr);

		#[allow(dead_code)]
		impl $name {
			pub const fn new(value: $repr) -> Self {
				Self(value & Self::ALL.0)
			}

			pub const fn value(self) -> $repr {
				self.0
			}

			pub const fn name(self) -> Option<&'static str> {
				match self {
					$(
						Self::$variant => Some($variant_name),
					)*
					_ => None,
				}
			}

			pub const fn contains(self, other: Self) -> bool {
				(self.0 & other.0) == other.0
			}

			pub const NONE: Self = Self(0);

			$(
				$(#[$variant_meta])*
				pub const $variant: Self = Self($value);
			)*

			const ALL: Self = Self(0 $(| Self::$variant.0)*);
		}

		impl ::std::str::FromStr for $name {
			type Err = $crate::bitflags::UnknownFlag;

			fn from_str(value: &str) -> ::std::result::Result<Self, <Self as ::std::str::FromStr>::Err> {
				match value {
					$(
						$variant_name => Ok(Self::$variant),
					)*
					unknown => Err($crate::bitflags::UnknownFlag(unknown.to_owned())),
				}
			}
		}

		impl ::std::ops::Deref for $name {
			type Target = $repr;

			fn deref(&self) -> &<Self as ::std::ops::Deref>::Target {
				&self.0
			}
		}

		impl ::std::ops::BitOr for $name {
			type Output = Self;

			fn bitor(self, rhs: Self) -> Self::Output {
				Self(self.0 | rhs.0)
			}
		}

		impl ::std::ops::BitOrAssign for $name {
			fn bitor_assign(&mut self, rhs: Self) {
				self.0 |= rhs.0;
			}
		}

		impl ::std::ops::BitAnd for $name {
			type Output = Self;

			fn bitand(self, rhs: Self) -> Self::Output {
				Self(self.0 & rhs.0)
			}
		}

		impl ::std::ops::BitAndAssign for $name {
			fn bitand_assign(&mut self, rhs: Self) {
				self.0 &= rhs.0;
			}
		}

		impl ::std::ops::BitXor for $name {
			type Output = Self;

			fn bitxor(self, rhs: Self) -> Self::Output {
				Self(self.0 ^ rhs.0)
			}
		}

		impl ::std::ops::BitXorAssign for $name {
			fn bitxor_assign(&mut self, rhs: Self) {
				self.0 ^= rhs.0;
			}
		}

		#[derive(Debug, Clone)]
		pub struct $iter_name {
			bits: $repr,
			idx: $repr,
		}

		impl $iter_name {
			const fn new(flags: $name) -> Self {
				Self { bits: flags.0, idx: 0 }
			}
		}

		impl ::std::iter::Iterator for $iter_name {
			type Item = &'static str;

			fn next(&mut self) -> Option<<Self as ::std::iter::Iterator>::Item> {
				if self.bits == 0 {
					return None;
				}

				if self.idx >= <$repr>::BITS {
					return None;
				}

				while self.bits != 0 && self.idx <= <$repr>::BITS {
					if let Some(name) = $name(self.bits & (1 << self.idx)).name() {
						self.idx += 1;
						return Some(name);
					}

					self.idx += 1;
				}

				None
			}
		}

		impl ::std::iter::IntoIterator for $name {
			type Item = &'static str;
			type IntoIter = $iter_name;

			fn into_iter(self) -> <Self as ::std::iter::IntoIterator>::IntoIter {
				$iter_name::new(self)
			}
		}

		impl ::std::fmt::Display for $name {
			fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
				f.debug_list().entries((*self).into_iter()).finish()
			}
		}

		impl ::serde::Serialize for $name {
			fn serialize<S>(&self, serializer: S) -> ::std::result::Result<S::Ok, S::Error>
			where
				S: ::serde::Serializer,
			{
				let mut serializer = serializer.serialize_seq(None)?;

				for value in *self {
					::serde::ser::SerializeSeq::serialize_element(&mut serializer, value)?;
				}

				::serde::ser::SerializeSeq::end(serializer)
			}
		}

		impl<'de> ::serde::Deserialize<'de> for $name {
			fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
			where
				D: ::serde::Deserializer<'de>,
			{
				#[derive(::serde::Deserialize)]
				#[serde(untagged)]
				enum Helper {
					Int($repr),
					Word(String),
					Words(Vec<String>),
				}

				Helper::deserialize(deserializer).map(|value| match value {
					Helper::Int(flags) => Self::new(flags),
					Helper::Word(word) => word
						.parse::<Self>()
						.unwrap_or_default(),
					Helper::Words(words) => words
						.into_iter()
						.flat_map(|word| word.parse::<Self>())
						.fold(Self::NONE, |acc, curr| (acc | curr))
				})
			}
		}
	};
}

pub(crate) use bitflags;

/// Indicates a failure when parsing the string representation of a flag created with
/// [`bitflags!()`].
#[derive(Debug, Error)]
#[error("unknown flag `{0}`")]
pub struct UnknownFlag(pub String);
