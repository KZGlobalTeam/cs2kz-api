// Copyright (C) AlphaKeks <alphakeks@dawn.sh>
//
// This is free software. You can redistribute it and / or modify it under the terms of the
// GNU General Public License as published by the Free Software Foundation, either version 3
// of the License, or (at your option) any later version.
//
// You should have received a copy of the GNU General Public License along with this repository.
// If not, see <https://www.gnu.org/licenses/>.

use {
	crate::config::Config,
	clap::Parser,
	std::{net::Ipv4Addr, path::PathBuf},
};

#[derive(Parser)]
pub struct Args {
	/// Which config file to use.
	///
	/// An example config file is located in `@/configs/api.example.toml`.
	/// See [`Config`] for more information.
	#[arg(short, long = "config")]
	#[clap(default_value = "./configs/api.toml")]
	pub config_path: PathBuf,

	/// The IP address to run the API on.
	#[arg(short, long)]
	pub address: Option<Ipv4Addr>,

	/// The port to expose the API on.
	#[arg(short, long)]
	pub port: Option<u16>,

	/// Custom database URL.
	///
	/// Should be a MySQL connection string following this format:
	///
	/// mysql://user:password@host:port/database
	#[arg(long)]
	pub database_url: Option<String>,

	/// Enable logging.
	///
	/// The log level is controlled by the `RUST_LOG` environment variable.
	#[arg(long = "logs")]
	pub enable_logging: bool,
}

impl Args {
	/// Gets the CLI arguments for the current process.
	pub fn get() -> Args {
		// `Args::parse` requires `clap::Parser` to be in scope, hence this wrapper function.
		Args::parse()
	}

	/// Overrides any [`Config`] values with specified CLI arguments.
	pub fn override_config(&self, config: &mut Config) {
		if let Some(address) = self.address {
			config.address = address;
		}

		if let Some(port) = self.port {
			config.port = port;
		}

		if let Some(ref database_url) = self.database_url {
			config.database_url = database_url.clone();
		}

		if self.enable_logging {
			config.enable_logging = true;
		}
	}
}
