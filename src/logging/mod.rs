use tracing::info;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::Registry;

mod stderr;

pub fn init() {
	let registry = Registry::default().with(stderr::layer());

	registry.init();

	info! {
		tokio_console = cfg!(feature = "console"),
		"Initialized logging",
	};
}
