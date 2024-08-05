//! This module contains a [`tower::Service`] for authenticating requests using
//! JWTs. It will extract the `Authorization: Bearer …` header from the request
//! and decode it using a secret provided as part of the API configuration.
//!
//! See [module-level documentation] for more details about session
//! authentication in general.
//!
//! [module-level documentation]: crate::services::auth::session

use std::fmt;
use std::marker::PhantomData;
use std::task::{self, Poll};

use axum::extract::Request;
use axum::response::{IntoResponse, Response};
use axum::RequestExt;
use futures::future::BoxFuture;
use serde::de::DeserializeOwned;
use tap::Pipe;
use thiserror::Error;

use super::{Jwt, JwtRejection};
use crate::services::AuthService;

/// A layer producing the [`JwtService`] middleware.
pub struct JwtLayer<T>
where
	T: DeserializeOwned,
{
	/// For decoding JWTs.
	auth_svc: AuthService,

	/// The payload type of the JWTs we're gonna be decoding.
	_marker: PhantomData<T>,
}

impl<T> JwtLayer<T>
where
	T: DeserializeOwned,
{
	/// Creates a new [`JwtLayer`].
	pub fn new(auth_svc: AuthService) -> Self
	{
		Self { auth_svc, _marker: PhantomData }
	}
}

// We implement `Clone` manually here because a derived impl would also require
// `T: Clone`, which isn't necessary for this type.
impl<T> Clone for JwtLayer<T>
where
	T: DeserializeOwned,
{
	fn clone(&self) -> Self
	{
		Self::new(self.auth_svc.clone())
	}
}

impl<S, T> tower::Layer<S> for JwtLayer<T>
where
	T: DeserializeOwned,
{
	type Service = JwtService<S, T>;

	fn layer(&self, inner: S) -> Self::Service
	{
		JwtService { auth_svc: self.auth_svc.clone(), inner, _marker: PhantomData }
	}
}

/// A middleware for extracting a JWT from request headers and validating it,
/// before passing on the request.
///
/// You can create an instance of this service using [`JwtLayer`].
pub struct JwtService<S, T>
where
	T: DeserializeOwned,
{
	/// For decoding JWTs.
	auth_svc: AuthService,

	/// The inner service.
	inner: S,

	/// The payload type of the JWTs we're gonna be decoding.
	_marker: PhantomData<T>,
}

impl<S, T> Clone for JwtService<S, T>
where
	S: Clone,
	T: DeserializeOwned,
{
	fn clone(&self) -> Self
	{
		Self { auth_svc: self.auth_svc.clone(), inner: self.inner.clone(), _marker: PhantomData }
	}
}

/// Errors that can occur in the [`JwtService`] middleware.
#[derive(Debug, Error)]
pub enum JwtServiceError<S>
where
	S: tower::Service<Request, Response = Response>,
	S::Error: std::error::Error + IntoResponse,
{
	/// We failed to extract the JWT.
	#[error(transparent)]
	Jwt(#[from] JwtRejection),

	/// The underlying service failed for some reason.
	#[error(transparent)]
	Service(S::Error),
}

impl<S> IntoResponse for JwtServiceError<S>
where
	S: tower::Service<Request, Response = Response>,
	S::Error: std::error::Error + IntoResponse,
{
	fn into_response(self) -> Response
	{
		match self {
			Self::Jwt(source) => source.into_response(),
			Self::Service(source) => source.into_response(),
		}
	}
}

impl<S, T> tower::Service<Request> for JwtService<S, T>
where
	S: tower::Service<Request, Response = Response> + fmt::Debug + Clone + Send + 'static,
	S::Future: Send,
	S::Error: std::error::Error + IntoResponse,
	T: fmt::Debug + Clone + DeserializeOwned + Send + Sync + 'static,
{
	type Response = Response;
	type Error = JwtServiceError<S>;
	type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

	fn poll_ready(&mut self, cx: &mut task::Context<'_>) -> Poll<Result<(), Self::Error>>
	{
		self.inner.poll_ready(cx).map_err(JwtServiceError::Service)
	}

	fn call(&mut self, req: Request) -> Self::Future
	{
		let auth_svc = self.auth_svc.clone();
		let inner = self.inner.clone();

		Box::pin(svc_impl::<S, T>(auth_svc, inner, req))
	}
}

/// The relevant implementation of `<JwtService as tower::Service>::call()`.
#[tracing::instrument(level = "debug", skip(inner), err(Debug, level = "debug"))]
async fn svc_impl<S, T>(
	auth_svc: AuthService,
	mut inner: S,
	mut req: Request,
) -> Result<Response, JwtServiceError<S>>
where
	S: tower::Service<Request, Response = Response> + fmt::Debug + Clone + Send + 'static,
	S::Future: Send,
	S::Error: fmt::Debug + std::error::Error + IntoResponse,
	T: fmt::Debug + Clone + DeserializeOwned + Send + Sync + 'static,
{
	req.extract_parts_with_state::<Jwt<T>, _>(&auth_svc)
		.await?
		.pipe(|jwt| req.extensions_mut().insert(jwt));

	tracing::trace!("extracted JWT from request");

	let response = inner.call(req).await.map_err(JwtServiceError::Service)?;

	Ok(response)
}

#[cfg(test)]
mod tests
{
	use std::convert::Infallible;
	use std::time::Duration;

	use serde::{Deserialize, Serialize};
	use sqlx::{MySql, Pool};
	use tower::{service_fn, Layer, ServiceExt};

	use super::*;
	use crate::testing;

	#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
	struct TestInfo
	{
		foo: i32,
		bar: bool,
	}

	#[sqlx::test]
	async fn it_works(database: Pool<MySql>) -> color_eyre::Result<()>
	{
		let auth_svc = testing::auth_svc(database);
		let info = TestInfo { foo: 69, bar: true };
		let expires_after = Duration::from_secs(69);
		let jwt = auth_svc.encode_jwt(Jwt::new(&info, expires_after))?;

		let req = Request::builder()
			.method(http::Method::GET)
			.uri("/")
			.header("Authorization", format!("Bearer {jwt}"))
			.body(Default::default())?;

		let res = JwtLayer::<TestInfo>::new(auth_svc)
			.layer(service_fn(|req: Request| async move {
				assert!(req.extensions().get::<Jwt<TestInfo>>().is_some());
				Result::<_, Infallible>::Ok(Default::default())
			}))
			.oneshot(req)
			.await;

		testing::assert!(res.is_ok());

		Ok(())
	}
}
