use std::sync::Arc;
use std::{env, io};

use serde_json::Value as JsonValue;
use sqlx::MySqlConnection;
use time::macros::format_description;
use tokio::task;
use tracing::{error, info};
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::fmt::time::UtcTime;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer};

pub fn init(database: MySqlConnection, axiom_dataset: String) {
	let format = format_description!("[hour]:[second].[subsecond digits:5]");
	let timer = UtcTime::new(format);
	let span_events = FmtSpan::NEW | FmtSpan::CLOSE;
	let filter = EnvFilter::from_default_env();
	let level = filter.to_string();

	let stderr = tracing_subscriber::fmt::layer()
		.pretty()
		.with_thread_ids(true)
		.with_writer(io::stderr)
		.with_timer(timer)
		.with_span_events(span_events.clone())
		.with_filter(filter);

	let audit_logs = cs2kz_api::audit_logs::AuditLayer::new(database);

	let axiom_writer = AxiomWriter::new(axiom_dataset);
	let axiom_filter = env::var("AXIOM_FILTER")
		.map(EnvFilter::new)
		.unwrap_or_else(|_| EnvFilter::new("cs2kz_api=trace,axum=trace,sqlx=trace"));

	let axiom = tracing_subscriber::fmt::layer()
		.json()
		.with_level(true)
		.with_thread_ids(true)
		.with_writer(axiom_writer)
		.with_filter(axiom_filter);

	tracing_subscriber::registry()
		.with(stderr)
		.with(audit_logs)
		.with(axiom)
		.init();

	info!(%level, "Initialized logging");
}

struct AxiomWriter {
	dataset: String,
	client: Option<Arc<axiom_rs::Client>>,
}

impl AxiomWriter {
	fn new(dataset: String) -> Arc<Self> {
		let client = axiom_rs::Client::new()
			.map(Arc::new)
			.map_err(|err| {
				eprintln!("Failed to connect to Axiom: {err}");
				err
			})
			.ok();

		Arc::new(Self { dataset, client })
	}
}

impl io::Write for &AxiomWriter {
	fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
		let Some(client) = self.client.as_ref().map(Arc::clone) else {
			return Ok(0);
		};

		let dataset = self.dataset.clone();
		let data = serde_json::from_slice::<JsonValue>(buf).expect("invalid json logs");

		task::spawn(async move {
			if let Err(error) = client.ingest(dataset, [data]).await {
				error!(audit = true, ?error, "failed to send logs to axiom");
			}
		});

		Ok(buf.len())
	}

	fn flush(&mut self) -> io::Result<()> {
		Ok(())
	}
}
