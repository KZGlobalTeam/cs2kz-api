use audit_logs::AuditLogs;
use sqlx::MySqlPool;
use tracing::info;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::Registry;

mod stderr;
mod log;
mod audit_logs;

pub fn init(audit_log_db: MySqlPool) {
	let registry = Registry::default()
		.with(stderr::layer())
		.with(AuditLogs::layer(audit_log_db));

	registry.init();

	info! {
		tokio_console = cfg!(feature = "console"),
		"Initialized logging",
	};
}
