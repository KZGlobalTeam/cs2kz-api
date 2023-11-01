use {crate::config::Config, clap::Parser, std::net::Ipv4Addr};

#[derive(Parser)]
pub struct Args {
	/// The IP address to run the API on.
	#[arg(short, long)]
	pub address: Option<Ipv4Addr>,

	/// The port to expose the API on.
	#[arg(short, long)]
	pub port: Option<u16>,

	/// Enable logging.
	///
	/// The log level is controlled by the `RUST_LOG` environment variable.
	#[arg(long = "logs")]
	pub enable_logging: bool,

	/// Custom database URL.
	///
	/// Should be a MySQL connection string following this format:
	///
	/// mysql://user:password@host:port/database
	#[arg(long)]
	pub database_url: Option<String>,
}

impl Args {
	/// Gets the CLI arguments for the current process.
	pub fn get() -> Self {
		Self::parse()
	}

	/// Overrides any [`Config`] values with specified CLI arguments.
	pub fn override_config(&self, config: &mut Config) {
		if let Some(address) = self.address {
			config.address = address;
		}

		if let Some(port) = self.port {
			config.port = port;
		}

		if self.enable_logging {
			config.enable_logging = true;
		}

		if let Some(ref database_url) = self.database_url {
			config.database_url = database_url.clone();
		}
	}
}
