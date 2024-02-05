use cs2kz_api::config::axiom;
use reqwest::header::{self, HeaderMap, HeaderValue};
use reqwest::Response;
use tokio::task;
use tracing::error;

use super::{layer, Log};

pub struct Client {
	config: axiom::Config,
	http_client: reqwest::Client,
}

impl Client {
	pub fn new(config: axiom::Config) -> Self {
		let headers = HeaderMap::from_iter([
			(header::CONTENT_TYPE, HeaderValue::from_static("application/json")),
			(
				header::AUTHORIZATION,
				HeaderValue::try_from(format!("Bearer {}", config.token)).unwrap(),
			),
		]);

		let http_client = reqwest::Client::builder()
			.default_headers(headers)
			.build()
			.unwrap();

		Self { config, http_client }
	}
}

impl layer::Consumer for Client {
	fn is_interested_in(metadata: &tracing::Metadata<'_>) -> bool {
		metadata.target().starts_with("cs2kz_api")
	}

	fn would_consume(log: &Log) -> bool {
		!log.fields.is_empty()
	}

	fn consume(&self, logs: Vec<Log>) {
		if logs.is_empty() {
			return;
		}

		let url = self.config.url.clone();
		let bytes = serde_json::to_vec(&logs).expect("invalid logs");
		let request = self.http_client.post(url).body(bytes);

		task::spawn(async move {
			if let Err(error) = request.send().await.and_then(Response::error_for_status) {
				error!(target: "audit_log", %error, "failed to send log to axiom");
			}
		});
	}
}
