use std::fs;
use std::path::Path;

use anyhow::Context;
use cs2kz_api::config::TracingConfig;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::Rotation;
use tracing_subscriber::filter::EnvFilter;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::layer::{Layer as _, SubscriberExt};
use tracing_subscriber::util::SubscriberInitExt;

mod cli;

fn main() -> anyhow::Result<()> {
    let cli_args = cli::args();
    let mut config = if let Some(config_path) = cli_args.config_path.as_deref() {
        read_and_parse_config_file(config_path)?
    } else if fs::exists("./cs2kz-api.toml")? {
        read_and_parse_config_file(Path::new("./cs2kz-api.toml"))?
    } else {
        cs2kz_api::Config::default()
    };

    cli_args.apply_to_config(&mut config);

    if config.tracing.enable {
        init_tracing(&config.tracing).context("failed to initialize tracing")?;
    }

    cs2kz_api::run(config).context("failed to run API")
}

fn read_and_parse_config_file(path: &Path) -> anyhow::Result<cs2kz_api::Config> {
    fs::read_to_string(path)
        .context("failed to read configuration file")
        .and_then(|text| toml::from_str(&text).context("failed to parse configuration file"))
}

fn init_tracing(config: &TracingConfig) -> anyhow::Result<Option<WorkerGuard>> {
    assert!(config.enable);

    let env_filter =
        || EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("cs2kz=info,warn"));

    let stderr = config.stderr.enable.then(|| {
        tracing_subscriber::fmt::layer()
            .pretty()
            .with_ansi(config.stderr.ansi)
    });

    let (files, guard) = config
        .files
        .enable
        .then(|| {
            if !config.files.directory.exists() {
                fs::create_dir_all(&config.files.directory).context("create log dir")?;
            }

            let log_dir = config
                .files
                .directory
                .canonicalize()
                .context("canonicalize log dir path")?;

            let (writer, guard) = tracing_appender::rolling::Builder::new()
                .rotation(Rotation::DAILY)
                .filename_suffix("log")
                .build(&log_dir)
                .map(tracing_appender::non_blocking)
                .context("failed to initialize logger")?;

            let layer = tracing_subscriber::fmt::layer()
                .compact()
                .with_ansi(false)
                .with_file(true)
                .with_level(true)
                .with_line_number(true)
                .with_span_events(FmtSpan::FULL)
                .with_target(true)
                .with_thread_ids(true)
                .with_thread_names(true)
                .with_writer(writer);

            anyhow::Ok((layer, guard))
        })
        .transpose()?
        .unzip();

    #[cfg(target_os = "linux")]
    let journald = config
        .journald
        .enable
        .then(|| {
            let mut layer =
                tracing_journald::layer().context("failed to initialize journald logger")?;

            if let Some(ref syslog_identifier) = config.journald.syslog_identifier {
                layer = layer.with_syslog_identifier(syslog_identifier.clone());
            }

            if let Some(ref field_prefix) = config.journald.field_prefix {
                layer = layer.with_field_prefix(Some(field_prefix.clone()));
            }

            anyhow::Ok(layer)
        })
        .transpose()?;

    let console = config.console.enable.then(|| {
        console_subscriber::ConsoleLayer::builder()
            .server_addr(config.console.server_addr.clone())
            .spawn()
    });

    let layers = tracing_subscriber::Layer::and_then(stderr, files);

    #[cfg(target_os = "linux")]
    let layers = tracing_subscriber::Layer::and_then(layers, journald);

    tracing_subscriber::registry()
        .with(layers.with_filter(env_filter()))
        .with(console)
        .init();

    Ok(guard)
}
