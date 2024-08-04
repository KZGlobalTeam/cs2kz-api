//! This module contains a function that will install a global panic hook.
//!
//! See [`std::panic::set_hook()`] for more details.

use std::backtrace::Backtrace;
use std::panic;

/// Installs the API's custom global panic hook.
///
/// The previous hook will be invoked after the API's custom hook.
#[tracing::instrument(target = "cs2kz_api::runtime", name = "panic_hook")]
pub fn install()
{
	let old_hook = panic::take_hook();

	panic::set_hook(Box::new(move |info| {
		tracing::error_span!(target: "cs2kz_api::runtime", "panic_hook").in_scope(|| {
			let backtrace = Backtrace::force_capture();

			tracing::error! {
				target: "cs2kz_api::audit_log",
				"\n{info}\n---\nbacktrace:\n{backtrace}",
			};
		});

		old_hook(info)
	}));

	tracing::info!("installed panic hook");
}
