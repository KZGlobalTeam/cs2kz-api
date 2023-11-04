macro_rules! id_or_name {
	($name:ident) => {
		use std::{borrow::Cow, fmt::Display};

		#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
		#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
		#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
		#[cfg_attr(feature = "serde", serde(untagged))]
		pub enum $name<'a> {
			ID(u16),
			Name(Cow<'a, str>),
		}

		impl<'a> $name<'a> {
			#[inline]
			pub fn name<S>(name: S) -> Self
			where
				S: Into<Cow<'a, str>>, {
				Self::Name(name.into())
			}
		}

		impl From<u16> for $name<'_> {
			fn from(id: u16) -> Self {
				Self::ID(id)
			}
		}

		impl<'a> From<&'a str> for $name<'a> {
			fn from(value: &'a str) -> Self {
				Self::Name(Cow::Borrowed(value))
			}
		}

		impl<'a> From<String> for $name<'a> {
			fn from(value: String) -> Self {
				Self::Name(Cow::Owned(value))
			}
		}

		impl Display for $name<'_> {
			fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
				match self {
					Self::ID(id) => write!(f, "{id}"),
					Self::Name(name) => write!(f, "{name}"),
				}
			}
		}
	};
}

pub(crate) use id_or_name;
