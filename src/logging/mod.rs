use std::error::Error as StdError;
use std::io;

use cs2kz_api::config::axiom::Config as AxiomConfig;
use cs2kz_api::config::database;
use time::macros::format_description;
use tracing::info;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::fmt::time::UtcTime;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer};

mod audit;
mod axiom;

pub async fn init(
	database_config: &database::Config,
	axiom_config: Option<AxiomConfig>,
) -> Result<(), Box<dyn StdError>> {
	let timer = UtcTime::new(format_description!("[hour]:[second].[subsecond digits:5]"));
	let span_events = FmtSpan::NEW | FmtSpan::CLOSE;
	let env_filter = EnvFilter::from_default_env();
	let log_level = env_filter.to_string();

	let stderr = tracing_subscriber::fmt::layer()
		.pretty()
		.with_timer(timer)
		.with_thread_ids(true)
		.with_thread_names(true)
		.with_span_events(span_events)
		.with_writer(io::stderr)
		.with_filter(env_filter);

	let audit_logs = audit::Layer::new(database_config.url.as_str()).await?;

	let axiom_filter = axiom_config
		.as_ref()
		.map(|config| config.log_filter.clone())
		.unwrap_or_else(|| String::from("RUST_LOG"));

	let axiom_writer = axiom_config.map(axiom::Writer::new).unwrap_or_default();

	let axiom = tracing_subscriber::fmt::layer()
		.json()
		.with_thread_ids(true)
		.with_writer(axiom_writer)
		.with_filter(EnvFilter::new(axiom_filter));

	let registry = tracing_subscriber::registry()
		.with(stderr)
		.with(audit_logs)
		.with(axiom);

	#[cfg(feature = "console")]
	let registry = registry.with(console_subscriber::spawn());

	registry.init();

	info!(%log_level, "Initialized logging");

	Ok(())
}
