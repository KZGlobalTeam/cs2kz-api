use std::backtrace::{Backtrace, BacktraceStatus};
use std::panic;

use super::{Environment, environment};

/// Sets the global [panic hook](std::panic::set_hook).
pub fn install() {
    panic::update_hook(|old_hook, panic_info| {
        match (panic_info.location(), panic_info.payload_as_str(), environment()) {
            (None, None, Environment::Local) => {
                tracing::error!(
                    target: "cs2kz_api::panics",
                    backtrace = %Backtrace::force_capture(),
                    "thread panicked",
                );
            },
            (None, None, Environment::Staging | Environment::Production) => {
                let backtrace = Backtrace::force_capture();

                if backtrace.status() == BacktraceStatus::Captured {
                    tracing::error!(target: "cs2kz_api::panics", %backtrace, "thread panicked");
                } else {
                    tracing::error!(target: "cs2kz_api::panics", "thread panicked");
                }
            },
            (None, Some(panic_message), Environment::Local) => {
                tracing::error!(
                    target: "cs2kz_api::panics",
                    backtrace = %Backtrace::force_capture(),
                    "thread panicked: {panic_message}",
                );
            },
            (None, Some(panic_message), Environment::Staging | Environment::Production) => {
                let backtrace = Backtrace::force_capture();

                if backtrace.status() == BacktraceStatus::Captured {
                    tracing::error!(
                        target: "cs2kz_api::panics",
                        %backtrace,
                        "thread panicked: {panic_message}",
                    );
                } else {
                    tracing::error!(
                        target: "cs2kz_api::panics",
                        "thread panicked: {panic_message}",
                    );
                }
            },
            (Some(location), None, Environment::Local) => {
                tracing::error!(
                    target: "cs2kz_api::panics",
                    %location,
                    backtrace = %Backtrace::force_capture(),
                    "thread panicked",
                );
            },
            (Some(location), None, Environment::Staging | Environment::Production) => {
                let backtrace = Backtrace::force_capture();

                if backtrace.status() == BacktraceStatus::Captured {
                    tracing::error!(
                        target: "cs2kz_api::panics",
                        %location,
                        %backtrace,
                        "thread panicked",
                    );
                } else {
                    tracing::error!(
                        target: "cs2kz_api::panics",
                        %location,
                        "thread panicked",
                    );
                }
            },
            (Some(location), Some(panic_message), Environment::Local) => {
                tracing::error!(
                    target: "cs2kz_api::panics",
                    %location,
                    backtrace = %Backtrace::force_capture(),
                    "thread panicked: {panic_message}",
                );
            },
            (
                Some(location),
                Some(panic_message),
                Environment::Staging | Environment::Production,
            ) => {
                let backtrace = Backtrace::force_capture();

                if backtrace.status() == BacktraceStatus::Captured {
                    tracing::error!(
                        target: "cs2kz_api::panics",
                        %location,
                        %backtrace,
                        "thread panicked: {panic_message}",
                    );
                } else {
                    tracing::error!(
                        target: "cs2kz_api::panics",
                        %location,
                        "thread panicked: {panic_message}",
                    );
                }
            },
        }

        old_hook(panic_info)
    });
}
