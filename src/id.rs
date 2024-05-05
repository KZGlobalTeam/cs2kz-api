//! A helper macro for creating "ID" types.

/// Creates a thin integer wrapper that can be used as an ID with semantic meaning.
macro_rules! make_id {
	($name:ident as $repr:ty) => {
		#[allow(missing_docs, clippy::missing_docs_in_private_items)]
		#[repr(transparent)]
		#[derive(
			Debug,
			Clone,
			Copy,
			PartialEq,
			Eq,
			PartialOrd,
			Ord,
			Hash,
			::derive_more::Display,
			::serde::Serialize,
			::serde::Deserialize,
			::sqlx::Type,
			::utoipa::ToSchema,
		)]
		#[serde(transparent)]
		#[sqlx(transparent)]
		#[display("{_0}")]
		pub struct $name(pub $repr);

		impl ::std::ops::Deref for $name {
			type Target = $repr;

			fn deref(&self) -> &Self::Target {
				&self.0
			}
		}
	};
}

pub(crate) use make_id;
