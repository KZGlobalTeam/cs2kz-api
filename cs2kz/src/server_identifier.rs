use std::{borrow::Cow, fmt::Display};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
pub enum ServerIdentifier<'a> {
	ID(u16),
	Name(Cow<'a, str>),
}

impl<'a> ServerIdentifier<'a> {
	#[inline]
	pub fn name<S>(name: S) -> Self
	where
		S: Into<Cow<'a, str>>, {
		Self::Name(name.into())
	}
}

impl From<u16> for ServerIdentifier<'_> {
	fn from(id: u16) -> Self {
		Self::ID(id)
	}
}

impl<'a> From<&'a str> for ServerIdentifier<'a> {
	fn from(value: &'a str) -> Self {
		Self::Name(Cow::Borrowed(value))
	}
}

impl<'a> From<String> for ServerIdentifier<'a> {
	fn from(value: String) -> Self {
		Self::Name(Cow::Owned(value))
	}
}

impl Display for ServerIdentifier<'_> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			ServerIdentifier::ID(id) => write!(f, "{id}"),
			ServerIdentifier::Name(name) => write!(f, "{name}"),
		}
	}
}

#[cfg(feature = "serde")]
mod serde_impls {
	use {
		super::ServerIdentifier,
		serde::{Serialize, Serializer},
	};

	impl Serialize for ServerIdentifier<'_> {
		fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
		where
			S: Serializer, {
			match self {
				ServerIdentifier::ID(id) => id.serialize(serializer),
				ServerIdentifier::Name(name) => name.serialize(serializer),
			}
		}
	}
}
