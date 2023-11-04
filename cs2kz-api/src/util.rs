use {
	axum::{http::StatusCode, response::IntoResponse},
	serde::{Deserialize, Serialize},
	std::fmt::Display,
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
