//! A middleware that wraps a [`tower::Service`] whose `Error` isn't infallible,
//! but implements [`IntoResponse`].
//!
//! This is useful because [`axum`], our web framework, only accepts infallible
//! services. [`InfallibleLayer`] allows configuring a service stack that will
//! always produce a response and can integrate with axum.

use std::convert;
use std::future::Future;
use std::pin::Pin;
use std::task::{self, Poll};

use axum::extract::Request;
use axum::response::{IntoResponse, Response};

/// A layer producing the [`Infallible`] service.
///
/// # Example
///
/// ```
/// use axum::{routing, Router};
/// use cs2kz_api::middleware::InfallibleLayer;
/// use cs2kz_api::services::auth::session::SessionManagerLayer;
/// use cs2kz_api::services::AuthService;
/// use tower::ServiceBuilder;
///
/// fn foo(auth_svc: AuthService) -> Router
/// {
///     let stack = ServiceBuilder::new()
///         .layer(InfallibleLayer::new())
///         .layer(SessionManagerLayer::new(auth_svc)); // fallible!
///
///     Router::new()
///         .route("/", routing::get(|| async { "Hello, world!" }))
///         .route_layer(stack) // still works!
/// }
/// ```
#[derive(Clone)]
pub struct InfallibleLayer
{
	/// non-exhaustive
	_priv: (),
}

impl InfallibleLayer
{
	/// Creates a new [`InfallibleLayer`].
	pub fn new() -> Self
	{
		Self { _priv: () }
	}
}

impl<S> tower::Layer<S> for InfallibleLayer
{
	type Service = Infallible<S>;

	fn layer(&self, inner: S) -> Self::Service
	{
		Infallible { inner }
	}
}

/// A middleware that converts another service's `Error` to a [`Response`].
#[derive(Clone)]
pub struct Infallible<S>
{
	/// The inner service.
	inner: S,
}

impl<S> tower::Service<Request> for Infallible<S>
where
	S: tower::Service<Request, Response = Response>,
	S::Error: IntoResponse,
{
	type Response = Response;
	type Error = convert::Infallible;
	type Future = ResponseFuture<S::Future, S::Response, S::Error>;

	fn poll_ready(&mut self, cx: &mut task::Context<'_>) -> Poll<Result<(), Self::Error>>
	{
		assert!(task::ready!(self.inner.poll_ready(cx)).is_ok(), "axum handlers are always ready");
		Poll::Ready(Ok(()))
	}

	fn call(&mut self, req: Request) -> Self::Future
	{
		ResponseFuture(self.inner.call(req))
	}
}

/// Future for `<Infallible<S> as tower::Service>::Future`.
#[pin_project]
pub struct ResponseFuture<F, O, E>(#[pin] F)
where
	F: Future<Output = Result<O, E>>;

impl<F, O, E> Future for ResponseFuture<F, O, E>
where
	F: Future<Output = Result<O, E>>,
	O: IntoResponse,
	E: IntoResponse,
{
	type Output = Result<Response, convert::Infallible>;

	fn poll(self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> Poll<Self::Output>
	{
		self.project().0.poll(cx).map(|res| match res {
			Ok(v) => Ok(v.into_response()),
			Err(e) => Ok(e.into_response()),
		})
	}
}

#[cfg(test)]
mod tests
{
	use tower::{service_fn, Layer, ServiceExt};

	use super::*;

	#[tokio::test]
	async fn it_works() -> color_eyre::Result<()>
	{
		struct Whoops;

		impl IntoResponse for Whoops
		{
			fn into_response(self) -> Response
			{
				(http::StatusCode::IM_A_TEAPOT, "whoops").into_response()
			}
		}

		let req = Request::builder()
			.method(http::Method::GET)
			.uri("/")
			.body(Default::default())?;

		let res = InfallibleLayer::new()
			.layer(service_fn(|_| async { Err(Whoops) }))
			.oneshot(req)
			.await?;

		assert_eq!(res.status(), http::StatusCode::IM_A_TEAPOT);

		let body = axum::body::to_bytes(res.into_body(), usize::MAX).await?;

		assert_eq!(&body[..], b"whoops");

		Ok(())
	}
}
