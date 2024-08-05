//! This module contains a [`tower::Service`] for authenticating requests using
//! opaque API keys.
//!
//! See [module-level documentation] for more details about key authentication
//! in general.
//!
//! [module-level documentation]: crate::services::auth::api_key

use std::fmt;
use std::task::{self, Poll};

use axum::extract::Request;
use axum::response::{IntoResponse, Response};
use axum::RequestExt;
use axum_extra::headers::authorization::Bearer;
use axum_extra::headers::Authorization;
use axum_extra::typed_header::TypedHeaderRejection;
use axum_extra::TypedHeader;
use futures::future::BoxFuture;
use sqlx::{MySql, Pool};
use thiserror::Error;

use super::ApiKey;
use crate::http::problem_details::{self, IntoProblemDetails, ProblemType};
use crate::http::ProblemDetails;

/// A layer producing the [`ApiKeyService`] middleware.
#[derive(Clone)]
pub struct ApiKeyLayer
{
	/// The name of the key.
	key_name: &'static str,

	/// Database connection.
	database: Pool<MySql>,
}

impl ApiKeyLayer
{
	/// Creates a new [`ApiKeyLayer`].
	pub fn new(key_name: &'static str, database: Pool<MySql>) -> Self
	{
		Self { key_name, database }
	}
}

impl<S> tower::Layer<S> for ApiKeyLayer
{
	type Service = ApiKeyService<S>;

	fn layer(&self, inner: S) -> Self::Service
	{
		ApiKeyService { key_name: self.key_name, database: self.database.clone(), inner }
	}
}

/// A middleware for extracting API keys from request headers.
///
/// You can create an instance of this service using [`ApiKeyLayer`].
#[derive(Clone)]
pub struct ApiKeyService<S>
{
	/// Name of the API key to authenticate.
	key_name: &'static str,

	/// Database connection.
	database: Pool<MySql>,

	/// The inner service.
	inner: S,
}

/// Errors that can occur in the [`ApiKeyService`] middleware.
#[derive(Error)]
pub enum ApiKeyServiceError<S>
where
	S: tower::Service<Request, Response = Response>,
	S::Error: IntoProblemDetails,
{
	/// We failed to extract the API key header.
	#[error(transparent)]
	ExtractHeader(TypedHeaderRejection),

	/// We failed to parse the API key.
	#[error("failed to parse key: {0}")]
	ParseKey(uuid::Error),

	/// The API key was invalid.
	#[error("key is not valid")]
	InvalidKey,

	/// Something went wrong communicating with the database.
	#[error("something went wrong")]
	Database(#[from] sqlx::Error),

	/// The underlying service failed.
	#[error(transparent)]
	Service(S::Error),
}

// Derived impl adds a bound for `S: fmt::Debug` which we don't want or need.
impl<S> fmt::Debug for ApiKeyServiceError<S>
where
	S: tower::Service<Request, Response = Response>,
	S::Error: IntoProblemDetails,
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		match self {
			Self::ExtractHeader(source) => f.debug_tuple("ExtractHeader").field(source).finish(),
			Self::ParseKey(source) => f.debug_tuple("ParseKey").field(source).finish(),
			Self::InvalidKey => f.debug_tuple("InvalidKey").finish(),
			Self::Database(source) => f.debug_tuple("Database").field(source).finish(),
			Self::Service(source) => f.debug_tuple("Service").field(source).finish(),
		}
	}
}

impl<S> IntoProblemDetails for ApiKeyServiceError<S>
where
	S: tower::Service<Request, Response = Response>,
	S::Error: IntoProblemDetails,
{
	fn problem_type(&self) -> ProblemType
	{
		match self {
			Self::ExtractHeader(source) => source.problem_type(),
			Self::ParseKey(_) => ProblemType::InvalidHeader,
			Self::InvalidKey => ProblemType::Unauthorized,
			Self::Database(source) => source.problem_type(),
			Self::Service(source) => source.problem_type(),
		}
	}

	fn add_extension_members(&self, ext: &mut problem_details::ExtensionMembers)
	{
		if let Self::Service(source) = self {
			source.add_extension_members(ext);
		}
	}
}

impl<S> IntoResponse for ApiKeyServiceError<S>
where
	S: tower::Service<Request, Response = Response>,
	S::Error: IntoProblemDetails,
{
	fn into_response(self) -> Response
	{
		ProblemDetails::from(self).into_response()
	}
}

impl<S> tower::Service<Request> for ApiKeyService<S>
where
	S: tower::Service<Request, Response = Response> + fmt::Debug + Clone + Send + 'static,
	S::Future: Send,
	S::Error: IntoProblemDetails,
{
	type Response = Response;
	type Error = ApiKeyServiceError<S>;
	type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

	fn poll_ready(&mut self, cx: &mut task::Context<'_>) -> Poll<Result<(), Self::Error>>
	{
		self.inner
			.poll_ready(cx)
			.map_err(ApiKeyServiceError::Service)
	}

	fn call(&mut self, req: Request) -> Self::Future
	{
		let this = self.clone();

		Box::pin(svc_impl(this, req))
	}
}

/// The relevant implementation of `<ApiKeyService as tower::Service>::call()`.
#[tracing::instrument(level = "debug", skip(svc), err(Debug, level = "debug"))]
async fn svc_impl<S>(
	mut svc: ApiKeyService<S>,
	mut req: Request,
) -> Result<Response, ApiKeyServiceError<S>>
where
	S: tower::Service<Request, Response = Response> + fmt::Debug + Clone + Send + 'static,
	S::Future: Send,
	S::Error: IntoProblemDetails,
{
	let header: TypedHeader<Authorization<Bearer>> = req
		.extract_parts_with_state(&svc.database)
		.await
		.map_err(ApiKeyServiceError::ExtractHeader)?;

	tracing::trace!(value = %header.token(), "extracted key from request");

	let key = header
		.token()
		.parse::<ApiKey>()
		.map_err(ApiKeyServiceError::ParseKey)?;

	let key_name = sqlx::query_scalar! {
		r"
		SELECT
		  name
		FROM
		  Credentials
		WHERE
		  `key` = ?
		  AND expires_on > NOW()
		LIMIT
		  1
		",
		key,
	}
	.fetch_optional(&svc.database)
	.await?
	.ok_or(ApiKeyServiceError::InvalidKey)?;

	tracing::trace!(name = key_name, "found key in database");

	if key_name != svc.key_name {
		return Err(ApiKeyServiceError::InvalidKey);
	}

	req.extensions_mut().insert(key);

	tracing::trace!(?key, "authenticated API key, calling inner service");

	svc.inner
		.call(req)
		.await
		.map_err(ApiKeyServiceError::Service)
}

#[cfg(test)]
mod tests
{
	use std::convert::Infallible;

	use sqlx::{MySql, Pool};
	use tower::{service_fn, Layer, ServiceExt};

	use super::*;
	use crate::testing;

	#[sqlx::test(
		migrations = "database/migrations",
		fixtures("../../../../database/fixtures/api-key.sql")
	)]
	async fn accept_valid_key(database: Pool<MySql>) -> color_eyre::Result<()>
	{
		let req = Request::builder()
			.method(http::Method::GET)
			.uri("/")
			.header("Authorization", "Bearer 00000000-0000-0000-0000-000000000000")
			.body(Default::default())?;

		let res = ApiKeyLayer::new("valid-key", database)
			.layer(service_fn(|_| async { Result::<_, Infallible>::Ok(Default::default()) }))
			.oneshot(req)
			.await;

		testing::assert!(res.is_ok());

		Ok(())
	}

	#[sqlx::test]
	async fn reject_missing_header(database: Pool<MySql>) -> color_eyre::Result<()>
	{
		let req = Request::builder()
			.method(http::Method::GET)
			.uri("/")
			.body(Default::default())?;

		let res = ApiKeyLayer::new("invalid", database)
			.layer(service_fn(|_| async { Result::<_, Infallible>::Ok(Default::default()) }))
			.oneshot(req)
			.await
			.unwrap_err();

		testing::assert_matches!(res, ApiKeyServiceError::ExtractHeader(ref rej) if rej.is_missing());

		Ok(())
	}

	#[sqlx::test]
	async fn reject_invalid_header(database: Pool<MySql>) -> color_eyre::Result<()>
	{
		let req = Request::builder()
			.method(http::Method::GET)
			.uri("/")
			.header("Authorization", "foobarbaz")
			.body(Default::default())?;

		let res = ApiKeyLayer::new("invalid", database)
			.layer(service_fn(|_| async { Result::<_, Infallible>::Ok(Default::default()) }))
			.oneshot(req)
			.await
			.unwrap_err();

		testing::assert_matches!(res, ApiKeyServiceError::ExtractHeader(ref rej) if !rej.is_missing());

		Ok(())
	}

	#[sqlx::test]
	async fn reject_malformed_key(database: Pool<MySql>) -> color_eyre::Result<()>
	{
		let req = Request::builder()
			.method(http::Method::GET)
			.uri("/")
			.header("Authorization", "Bearer not-a-uuid")
			.body(Default::default())?;

		let res = ApiKeyLayer::new("invalid", database)
			.layer(service_fn(|_| async { Result::<_, Infallible>::Ok(Default::default()) }))
			.oneshot(req)
			.await
			.unwrap_err();

		testing::assert_matches!(res, ApiKeyServiceError::ParseKey(_));

		Ok(())
	}

	#[sqlx::test(migrations = "database/migrations")]
	async fn reject_invalid_key(database: Pool<MySql>) -> color_eyre::Result<()>
	{
		let req = Request::builder()
			.method(http::Method::GET)
			.uri("/")
			.header("Authorization", "Bearer 00000000-0000-0000-0000-000000000000")
			.body(Default::default())?;

		let res = ApiKeyLayer::new("invalid", database)
			.layer(service_fn(|_| async { Result::<_, Infallible>::Ok(Default::default()) }))
			.oneshot(req)
			.await
			.unwrap_err();

		testing::assert_matches!(res, ApiKeyServiceError::InvalidKey);

		Ok(())
	}
}
