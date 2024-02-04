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
use tracing_subscriber::{EnvFilter, Layer as _};

use self::layer::Layer;

mod visitor;
mod layer;

pub async fn init(
	database_config: &database::Config,
	axiom_config: Option<AxiomConfig>,
) -> Result<(), Box<dyn StdError>> {
	let timer = UtcTime::new(format_description!("[hour]:[second].[subsecond digits:5]"));
	let span_events = FmtSpan::NEW | FmtSpan::CLOSE;

	let stderr = tracing_subscriber::fmt::layer()
		.pretty()
		.with_timer(timer)
		.with_thread_ids(true)
		.with_thread_names(true)
		.with_span_events(span_events)
		.with_writer(io::stderr)
		.with_filter(EnvFilter::from_default_env());

	let custom = Layer::new(database_config.url.as_str(), axiom_config)
		.await?
		.with_filter(EnvFilter::from_default_env());

	let registry = tracing_subscriber::registry().with(stderr).with(custom);

	#[cfg(feature = "console")]
	let registry =
		registry.with(console_subscriber::spawn().with_filter(EnvFilter::new("tokio=trace")));

	registry.init();

	info! {
		filter = %EnvFilter::from_default_env(),
		tokio_console = cfg!(feature = "console"),
		"Initialized logging",
	};

	Ok(())
}
