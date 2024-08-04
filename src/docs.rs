//! This module contains the `/docs` endpoint.

use axum::response::{Html, IntoResponse};
use axum::{routing, Router};

use crate::http::problem_details::ProblemType;
use crate::openapi;

/// Returns a router for the `/docs` endpoint.
pub fn router() -> Router
{
	Router::new()
		.route("/docs/problem-types", routing::get(html))
		.route("/docs/static/problem-types.css", routing::get(css))
		.merge(openapi::Schema::swagger_ui())
}

/// Returns the HTML for the problem types page.
async fn html() -> Html<&'static str>
{
	Html(ProblemType::DOCS)
}

/// Returns the CSS for the problem types page.
async fn css() -> impl IntoResponse
{
	(
		http::StatusCode::OK,
		[("Content-Type", "text/css")],
		include_str!("../static/problem-types.css"),
	)
}
