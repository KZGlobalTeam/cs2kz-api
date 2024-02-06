use audit_logs::AuditLogs;
use sqlx::MySqlPool;
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

pub fn init(audit_log_db: MySqlPool) {
#[cfg(feature = "console")]
mod console;
	let registry = Registry::default()
		.with(stderr::layer())
		.with(AuditLogs::layer(audit_log_db));
	#[cfg(feature = "console")]
	let registry = registry.with(console::layer());

	registry.init();

	info!(tokio_console = cfg!(feature = "console"), "Initialized logging");
}
