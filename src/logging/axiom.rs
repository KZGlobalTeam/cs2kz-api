use cs2kz_api::audit;
use cs2kz_api::config::axiom::Config as AxiomConfig;
use reqwest::header::{self, HeaderMap, HeaderValue};
use reqwest::Response;
use tokio::task;
use tracing_subscriber::filter::FilterFn;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::Layer as _;

use crate::logging::layer::ConsumeLog;
use crate::logging::{Layer, Log};

/// Log layer for sending logs to https://axiom.co
pub struct Axiom {
	/// Config holding various details about the axiom dataset.
	config: AxiomConfig,

	/// HTTP client for sending the requests.
	http_client: reqwest::Client,
}

impl Axiom {
	pub fn layer<S>(config: AxiomConfig) -> impl tracing_subscriber::Layer<S>
	where
		S: tracing::Subscriber + for<'a> LookupSpan<'a>,
	{
		let default_headers = HeaderMap::from_iter([
			(header::CONTENT_TYPE, HeaderValue::from_static("application/json")),
			(
				header::AUTHORIZATION,
				HeaderValue::try_from(format!("Bearer {}", config.token)).unwrap(),
			),
		]);

		let http_client = reqwest::Client::builder()
			.default_headers(default_headers)
			.build()
			.expect("this is a valid client");

		Layer::new(Self { config, http_client }).with_filter(FilterFn::new(|metadata| {
			metadata.target().starts_with("cs2kz_api")
				&& metadata.fields().field("skip_axiom").is_none()
				&& !metadata.fields().is_empty()
		}))
	}
}

impl ConsumeLog for Axiom {
	fn consume_log(&'static self, log: Log) {
		let json = serde_json::to_vec(&[log]).expect("invalid logs");
		let request = self.http_client.post(self.config.url.clone()).body(json);

		task::spawn(async move {
			if let Err(error) = request.send().await.and_then(Response::error_for_status) {
				audit!(error, "failed sending log to axiom", %error);
			}
		});
	}
}
