use audit_logs::AuditLogs;
use axiom::Axiom;
use cs2kz_api::config::axiom::Config as AxiomConfig;
use sqlx::MySqlConnection;
use tracing::info;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::Registry;

mod log;
pub use log::Log;

mod layer;
pub use layer::Layer;

mod stderr;
mod audit_logs;
mod axiom;

#[cfg(feature = "console")]
mod console;

/// Initializes a [`Registry`] for collecting logs and distributing them to various outputs.
pub fn init(audit_log_db: MySqlConnection, axiom_config: Option<AxiomConfig>) {
	let registry = Registry::default()
		.with(stderr::layer())
		.with(AuditLogs::layer(audit_log_db))
		.with(axiom_config.map(Axiom::layer));

	#[cfg(feature = "console")]
	let registry = registry.with(console::layer());

	registry.init();

	info!(tokio_console = cfg!(feature = "console"), "Initialized logging");
}
