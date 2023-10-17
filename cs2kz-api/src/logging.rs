// Copyright (C) AlphaKeks <alphakeks@dawn.sh>
//
// This is free software. You can redistribute it and / or modify it under the terms of the
// GNU General Public License as published by the Free Software Foundation, either version 3
// of the License, or (at your option) any later version.
//
// You should have received a copy of the GNU General Public License along with this repository.
// If not, see <https://www.gnu.org/licenses/>.

//! Logging related functions.

use {
	tracing::info,
	tracing_subscriber::{fmt::format::FmtSpan, EnvFilter},
};

/// The default log level.
///
/// This will be used if `RUST_LOG` was not specified.
const DEFAULT_FILTER: &str = "WARN,cs2kz_api=TRACE";

/// Will initialize logging.
pub fn init() {
	// Write logs to STDERR.
	let writer = std::io::stderr;

	// Try to get `RUST_LOG` from the environment. Otherwise fall back to some default value.
	let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| DEFAULT_FILTER.into());
	let level = env_filter.to_string();

	// Which [`tracing::instrument`] events to log.
	let span_events = FmtSpan::ACTIVE;

	tracing_subscriber::fmt()
		.pretty()
		.with_writer(writer)
		.with_span_events(span_events)
		.with_env_filter(env_filter)
		.init();

	info!(%level, "Initialized logging");
}
