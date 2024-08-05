//! This module contains a [`tower::Service`] for authenticating requests using
//! sessions. It will extract a session from the request, authorize it, run its
//! inner service, and then extend the session, returning an updated cookie with
//! the response.
//!
//! See [module-level documentation] for more details about session
//! authentication in general.
//!
//! [module-level documentation]: crate::services::auth::session

use std::fmt;
use std::task::{self, Poll};

use axum::extract::Request;
use axum::response::{IntoResponse, Response};
use axum::RequestExt;
use futures::future::BoxFuture;
use http::header;
use thiserror::Error;

use super::{authorization, AuthorizeSession, Session, SessionRejection};
use crate::http::problem_details::IntoProblemDetails;
use crate::http::ProblemDetails;
use crate::services::AuthService;

/// A layer producing the [`SessionManager`] middleware.
#[derive(Clone)]
pub struct SessionManagerLayer<A = authorization::None>
{
	/// For database and API config access.
	auth_svc: AuthService,

	/// The authorization strategy.
	authorization: A,
}

impl SessionManagerLayer
{
	/// Creates a new [`SessionManagerLayer`].
	pub fn new(auth_svc: AuthService) -> Self
	{
		Self { auth_svc, authorization: authorization::None }
	}

	/// Creates a new [`SessionManagerLayer`] with an authorization strategy.
	pub fn with_strategy<A>(auth_svc: AuthService, authorization: A) -> SessionManagerLayer<A>
	{
		SessionManagerLayer { auth_svc, authorization }
	}
}

impl<S, A> tower::Layer<S> for SessionManagerLayer<A>
where
	A: AuthorizeSession,
{
	type Service = SessionManager<S, A>;

	fn layer(&self, inner: S) -> Self::Service
	{
		SessionManager {
			auth_svc: self.auth_svc.clone(),
			authorization: self.authorization.clone(),
			inner,
		}
	}
}

/// A middleware for authenticating & authorizing sessions.
///
/// You can create an instance of this service using [`SessionManagerLayer`].
#[derive(Clone)]
pub struct SessionManager<S, A = authorization::None>
{
	/// For database and API config access.
	auth_svc: AuthService,

	/// The authorization strategy.
	authorization: A,

	/// The inner service.
	inner: S,
}

/// Errors that can occur in the [`SessionManager`] middleware.
#[derive(Debug, Error)]
pub enum SessionManagerError<S, A = authorization::None>
where
	S: tower::Service<Request, Response = Response>,
	S::Error: IntoProblemDetails,
	A: AuthorizeSession,
	A::Error: IntoProblemDetails,
{
	/// Extracting the session failed.
	#[error(transparent)]
	Session(#[from] SessionRejection),

	/// Authorization failed.
	#[error(transparent)]
	Authorize(A::Error),

	/// The underlying service failed.
	#[error(transparent)]
	Service(S::Error),
}

impl<S, A> IntoResponse for SessionManagerError<S, A>
where
	S: tower::Service<Request, Response = Response>,
	S::Error: IntoProblemDetails,
	A: AuthorizeSession,
	A::Error: IntoProblemDetails,
{
	fn into_response(self) -> Response
	{
		match self {
			Self::Session(source) => ProblemDetails::from(source).into_response(),
			Self::Authorize(source) => ProblemDetails::from(source).into_response(),
			Self::Service(source) => ProblemDetails::from(source).into_response(),
		}
	}
}

impl<S, A> tower::Service<Request> for SessionManager<S, A>
where
	S: tower::Service<Request, Response = Response> + fmt::Debug + Clone + Send + 'static,
	S::Future: Send,
	S::Error: IntoProblemDetails,
	A: AuthorizeSession + fmt::Debug,
	A::Error: IntoProblemDetails,
{
	type Response = Response;
	type Error = SessionManagerError<S, A>;
	type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

	fn poll_ready(&mut self, cx: &mut task::Context<'_>) -> Poll<Result<(), Self::Error>>
	{
		self.inner
			.poll_ready(cx)
			.map_err(SessionManagerError::Service)
	}

	fn call(&mut self, req: Request) -> Self::Future
	{
		let auth_svc = self.auth_svc.clone();
		let authorization = self.authorization.clone();
		let inner = self.inner.clone();

		Box::pin(svc_impl(auth_svc, authorization, inner, req))
	}
}

/// The relevant implementation of `<SessionManager as tower::Service>::call()`.
#[tracing::instrument(level = "debug", skip(inner), err(Debug, level = "debug"))]
async fn svc_impl<A, S>(
	auth_svc: AuthService,
	authorization: A,
	mut inner: S,
	mut req: Request,
) -> Result<Response, SessionManagerError<S, A>>
where
	A: AuthorizeSession + fmt::Debug,
	A::Error: IntoProblemDetails,
	S: tower::Service<Request, Response = Response> + fmt::Debug + Clone + Send + 'static,
	S::Future: Send,
	S::Error: IntoProblemDetails,
{
	let session: Session = req.extract_parts_with_state(&auth_svc.database).await?;

	tracing::trace! {
		?session,
		auth_strategy = %std::any::type_name::<A>(),
		"extracted session from request; authorizing",
	};

	authorization
		.authorize_session(&session, &mut req)
		.await
		.map_err(SessionManagerError::Authorize)?;

	req.extensions_mut().insert(session.clone());

	tracing::trace!(?session, "authenticated and authorized session, calling inner service");

	let mut response = inner
		.call(req)
		.await
		.map_err(SessionManagerError::Service)?;

	let session_cookie = session
		.into_cookie(&*auth_svc.cookie_domain)
		.encoded()
		.to_string()
		.parse::<http::HeaderValue>()
		.expect("valid cookie");

	tracing::trace!(?session_cookie, "request complete; extending session");

	response
		.headers_mut()
		.insert(header::SET_COOKIE, session_cookie);

	Ok(response)
}

#[cfg(test)]
mod tests
{
	use std::convert::Infallible;

	use sqlx::{MySql, Pool};
	use tower::{service_fn, Layer, ServiceExt};

	use super::*;
	use crate::http::problem_details::{IntoProblemDetails, ProblemType};
	use crate::services::auth::session::{AuthorizeSession, SessionID};
	use crate::testing;

	#[derive(Debug, Clone)]
	struct YouShallNotPass;

	#[derive(Debug, thiserror::Error)]
	#[error("you shall not pass")]
	struct YouShallNotPassError;

	impl IntoProblemDetails for YouShallNotPassError
	{
		fn problem_type(&self) -> ProblemType
		{
			ProblemType::Unauthorized
		}
	}

	impl AuthorizeSession for YouShallNotPass
	{
		type Error = YouShallNotPassError;

		async fn authorize_session(self, _: &Session, _: &mut Request) -> Result<(), Self::Error>
		{
			Err(YouShallNotPassError)
		}
	}

	#[sqlx::test(
		migrations = "database/migrations",
		fixtures("../../../../database/fixtures/session.sql")
	)]
	async fn it_works(database: Pool<MySql>) -> color_eyre::Result<()>
	{
		let auth_svc = testing::auth_svc(database);

		let req = Request::builder()
			.method(http::Method::GET)
			.uri("/")
			.header("Cookie", format!("kz-auth={}", SessionID::TESTING))
			.body(Default::default())?;

		let res = SessionManagerLayer::new(auth_svc)
			.layer(service_fn(|_| async { Result::<_, Infallible>::Ok(Default::default()) }))
			.oneshot(req)
			.await;

		testing::assert!(res.is_ok());

		Ok(())
	}

	#[sqlx::test(
		migrations = "database/migrations",
		fixtures("../../../../database/fixtures/session.sql")
	)]
	async fn reject_unauthorized(database: Pool<MySql>) -> color_eyre::Result<()>
	{
		let auth_svc = testing::auth_svc(database);

		let req = Request::builder()
			.method(http::Method::GET)
			.uri("/")
			.header("Cookie", format!("kz-auth={}", SessionID::TESTING))
			.body(Default::default())?;

		let res = SessionManagerLayer::with_strategy(auth_svc, YouShallNotPass)
			.layer(service_fn(|_| async { Result::<_, Infallible>::Ok(Default::default()) }))
			.oneshot(req)
			.await
			.unwrap_err();

		testing::assert_matches!(res, SessionManagerError::Authorize(YouShallNotPassError));

		Ok(())
	}
}
