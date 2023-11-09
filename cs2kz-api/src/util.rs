use {
	axum::{http::StatusCode, response::IntoResponse},
	serde::{Deserialize, Deserializer, Serialize},
	std::fmt::Display,
	utoipa::ToSchema,
};

/// A filter to use in database queries.
///
/// Can be [`.push()`](sqlx::QueryBuilder::push)'ed to a query to concatenate filters. After
/// pushing, you can call [`.switch()`](Self::switch) so the next push will use [`Filter::And`].
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Filter {
	#[default]
	Where,

	And,
}

impl Filter {
	pub const fn new() -> Self {
		Self::Where
	}

	/// Switch `self` to [`Filter::And`].
	pub fn switch(&mut self) {
		*self = Self::And;
	}
}

impl Display for Filter {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str(match self {
			Filter::Where => " WHERE ",
			Filter::And => " AND ",
		})
	}
}

/// Wraps something such that a generated [`Response`](axum::response::Response) will have an
/// HTTP status code of 201.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Created<T>(pub T);

impl<T> IntoResponse for Created<T>
where
	T: IntoResponse,
{
	fn into_response(self) -> axum::response::Response {
		(StatusCode::CREATED, self).into_response()
	}
}

/// A utility type for deserializing a [`u64`].
///
/// * `DEFAULT`: the fallback value to be used if the actual value was null (defaults to 0)
/// * `MAX`: the maximum value that is allowed (defaults to [`u64::MAX`])
/// * `MIN`: the minimum value that is allowed (defaults to 0)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, ToSchema)]
pub struct BoundedU64<const DEFAULT: u64 = 0, const MAX: u64 = { u64::MAX }, const MIN: u64 = 0> {
	pub value: u64,
}

pub type Offset = BoundedU64;
pub type Limit<const LIMIT: u64> = BoundedU64<100, LIMIT>;

impl<'de, const DEFAULT: u64, const MAX: u64, const MIN: u64> Deserialize<'de>
	for BoundedU64<DEFAULT, MAX, MIN>
{
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>, {
		use serde::de::Error;

		let value = match Option::<u64>::deserialize(deserializer)? {
			None => DEFAULT,
			// No `Some(value @ MIN..=MAX)` pattern matching :(
			Some(value) if (MIN..=MAX).contains(&value) => value,
			Some(out_of_bounds) => {
				return Err(Error::custom(format!(
					"expected integer in the range of {MIN}..={MAX} but got {out_of_bounds}"
				)));
			}
		};

		Ok(Self { value })
	}
}
