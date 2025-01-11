#[cfg(not(feature = "fake"))]
compile_error!("you must run this binary with `--features fake`");

use std::env;
use std::num::NonZero;

use anyhow::Context as _;
use cs2kz::Context;
use cs2kz::config::{Config, DatabaseConfig};
use cs2kz::git::GitRevision;
use cs2kz::maps::CourseFilterId;
use cs2kz::pagination::Limit;
use cs2kz::players::{self, CreatePlayerError, NewPlayer};
use cs2kz::plugin::{self, NewPluginVersion, PluginVersionId};
use cs2kz::records::{self, NewRecord};
use cs2kz::servers::{self, ApproveServerError, GetServersParams};
use cs2kz::users::{self, CreateUserError, NewUser};
use fake::rand::prelude::SliceRandom;
use fake::{Fake, Faker};
use tokio::time::{Duration, interval};
use tracing::info;
use tracing_subscriber::EnvFilter;
use url::Url;

use self::cli::Resource;

mod cli;

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new("generator=trace,warn"))
        .init();

    let args = cli::args();

    let cfg = env::var("DATABASE_URL")
        .context("missing `DATABASE_URL` environment variable")?
        .parse::<Url>()
        .context("`DATABASE_URL` is not a valid URL")
        .map(|url| Config {
            database: DatabaseConfig {
                url,
                min_connections: 1,
                max_connections: Some(NonZero::<u32>::MIN),
            },
        })?;

    let cx = Context::new(cfg).await?;

    match args.resource {
        Resource::Records { clear, filter_id } => {
            if clear {
                records::clear(&cx, filter_id)
                    .await
                    .context("failed to clear records")?
            } else {
                generate_records(&cx, filter_id, args.count).await?
            }
        },
    }

    Ok(())
}

async fn generate_records(
    cx: &Context,
    filter_id: CourseFilterId,
    count: u64,
) -> anyhow::Result<()> {
    let plugin_version_id = get_or_create_plugin_version(cx).await?;
    let mut interval = interval(Duration::from_millis(10));

    let mut server_ids =
        servers::get(cx, GetServersParams { limit: Limit::MAX, ..Default::default() })
            .await?
            .map(|server| server.id)
            .collect::<Vec<_>>()
            .await?
            .into_inner();

    for _ in 0..count {
        interval.tick().await;

        let record = NewRecord {
            filter_id,
            server_id: 'outer: loop {
                if server_ids.len() >= 100 {
                    break *server_ids.choose(&mut fake::rand::thread_rng()).unwrap();
                }

                'inner: loop {
                    let server = Faker.fake::<cs2kz::servers::NewServer<'static>>();
                    let owner_name = fake::faker::name::en::Name().fake::<String>();

                    match users::create(cx, NewUser { id: server.owner_id, name: &owner_name })
                        .await
                    {
                        Ok(_) | Err(CreateUserError::UserAlreadyExists) => {},
                        Err(CreateUserError::Database(error)) => {
                            return Err(error).context("failed to create user");
                        },
                    }

                    match servers::approve(cx, server).await {
                        Ok((server_id, _)) => {
                            server_ids.push(server_id);
                            break 'outer server_id;
                        },
                        Err(ApproveServerError::OwnerDoesNotExist) => {
                            continue 'inner;
                        },
                        Err(
                            ApproveServerError::NameAlreadyTaken
                            | ApproveServerError::HostAndPortAlreadyTaken,
                        ) => {
                            continue 'outer;
                        },
                        Err(ApproveServerError::Database(error)) => {
                            return Err(error).context("failed to register player");
                        },
                    }
                }
            },
            plugin_version_id,
            ..Faker.fake()
        };

        match players::register(cx, NewPlayer { id: record.player_id, ..Faker.fake() }).await {
            Ok(_) | Err(CreatePlayerError::PlayerAlreadyExists) => {},
            Err(CreatePlayerError::Database(error)) => {
                return Err(error).context("failed to register player");
            },
        }

        let submitted = records::submit(cx, record)
            .await
            .context("failed to submit record")?;

        info!(id = %submitted.record_id, "submitted record");
    }

    Ok(())
}

async fn get_or_create_plugin_version(cx: &Context) -> anyhow::Result<PluginVersionId> {
    if let Some(version) = plugin::get_latest_version(cx).await? {
        return Ok(version.id);
    }

    let version = Faker.fake::<semver::Version>();
    let git_revision = Faker.fake::<GitRevision>();

    plugin::publish_version(cx, NewPluginVersion {
        version: &version,
        git_revision: &git_revision,
    })
    .await
    .context("failed to create plugin version")
}
