#![allow(unused_variables)]

#[cfg(not(feature = "fake"))]
compile_error!("you must run this binary with `--features fake`");

#[macro_use(info, warn)]
extern crate tracing;

use std::num::NonZero;
use std::{cmp, env, future};

use anyhow::Context as _;
use cs2kz::Context;
use cs2kz::config::{Config, DatabaseConfig};
use cs2kz::git::GitRevision;
use cs2kz::maps::courses::filters::{
    self,
    CourseFilterId,
    CourseFilterState,
    GetCourseFiltersParams,
    Tier,
};
use cs2kz::maps::{
    self,
    ApproveMapError,
    MapId,
    MapState,
    NewCourse,
    NewCourseFilter,
    NewCourseFilters,
    NewMap,
};
use cs2kz::pagination::Limit;
use cs2kz::players::{self, CreatePlayerError, GetPlayersParams, NewPlayer, PlayerId};
use cs2kz::plugin::{self, NewPluginVersion};
use cs2kz::records::{self, NewRecord, RecordId, SubmitRecordError, SubmittedRecord};
use cs2kz::servers::{self, ApproveServerError, GetServersParams, NewServer, ServerHost, ServerId};
use cs2kz::steam::WorkshopId;
use cs2kz::styles::Styles;
use cs2kz::users::{self, CreateUserError, NewUser, UserId};
use fake::faker::lorem::en::{Paragraph, Word};
use fake::faker::name::en::{FirstName, LastName};
use fake::rand::rngs::ThreadRng;
use fake::rand::seq::SliceRandom;
use fake::rand::{Rng, thread_rng};
use fake::{Fake, Faker};
use futures_util::future::Either;
use futures_util::stream::{self, StreamExt, TryStreamExt};
use steam_id::SteamId;
use tracing_subscriber::EnvFilter;
use url::Url;

use self::cli::{
    MapAction,
    PlayerAction,
    PluginVersionAction,
    RecordAction,
    Resource,
    ServerAction,
};

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
        Resource::PluginVersions {
            action: PluginVersionAction::Create { git_revision, version },
        } => create_plugin_version(&cx, &version, git_revision.as_ref())
            .await
            .context("failed to create plugin version"),
        Resource::PluginVersions {
            action: PluginVersionAction::Delete { version },
        } => delete_plugin_version(&cx, &version)
            .await
            .context("failed to delete plugin version"),
        Resource::Players { action: PlayerAction::Create { count } } => create_players(&cx, count)
            .await
            .context("failed to create players"),
        Resource::Players { action: PlayerAction::Delete { count } } => delete_players(&cx, count)
            .await
            .context("failed to delete players"),
        Resource::Servers {
            action: ServerAction::Create { owner_id, count },
        } => create_servers(&cx, owner_id, count)
            .await
            .context("failed to create servers"),
        Resource::Servers {
            action: ServerAction::Delete { owner_id, count },
        } => delete_servers(&cx, owner_id, count)
            .await
            .context("failed to delete servers"),
        Resource::Records {
            action:
                RecordAction::Create {
                    player_id,
                    server_id,
                    filter_id,
                    plugin_version,
                    count,
                },
        } => create_records(&cx, player_id, server_id, filter_id, plugin_version.as_ref(), count)
            .await
            .context("failed to create records"),
        Resource::Records {
            action: RecordAction::Delete { filter_id, starting_at, count },
        } => delete_records(&cx, filter_id, starting_at, count)
            .await
            .context("failed to delete records"),
        Resource::Maps { action: MapAction::Create { mappers, count } } => {
            create_maps(&cx, mappers, count)
                .await
                .context("failed to create maps")
        },
        Resource::Maps {
            action: MapAction::Delete { starting_at, count },
        } => delete_maps(&cx, starting_at, count)
            .await
            .context("failed to delete maps"),
    }
}

async fn create_plugin_version(
    cx: &Context,
    version: &semver::Version,
    git_revision: Option<&GitRevision>,
) -> anyhow::Result<()> {
    let mut fallback_git_revision = None;
    let git_revision =
        git_revision.unwrap_or_else(|| &*fallback_git_revision.get_or_insert(Faker.fake()));
    let id = plugin::publish_version(cx, NewPluginVersion { version, git_revision }).await?;

    info!(%id, "created plugin version");

    Ok(())
}

async fn delete_plugin_version(cx: &Context, version: &semver::Version) -> anyhow::Result<()> {
    if plugin::delete_version(cx, version).await? {
        info!("deleted plugin version");
    } else {
        anyhow::bail!("plugin version does not exist");
    }

    Ok(())
}

async fn create_players(cx: &Context, count: usize) -> anyhow::Result<()> {
    for player in (0..count).map(|_| Faker.fake::<NewPlayer<'static>>()) {
        let id = player.id;

        match players::register(cx, player).await {
            Ok(_) => info!(%id, "created player"),
            Err(CreatePlayerError::PlayerAlreadyExists) => warn!(%id, "player already exists"),
            Err(CreatePlayerError::Database(error)) => return Err(error.into()),
        }
    }

    Ok(())
}

async fn delete_players(cx: &Context, count: usize) -> anyhow::Result<()> {
    if let amount @ 1.. = players::delete(cx, count).await? {
        info!(amount, "deleted players");
    } else {
        anyhow::bail!("there are no players in the database")
    }

    Ok(())
}

async fn create_servers(
    cx: &Context,
    owner_id: Option<UserId>,
    count: usize,
) -> anyhow::Result<()> {
    const ALPHAKEKS_ID: SteamId = match SteamId::from_u64(76561198282622073_u64) {
        Ok(steam_id) => steam_id,
        Err(_) => unreachable!(),
    };

    if owner_id.is_none() {
        match users::create(cx, NewUser {
            id: UserId::new(ALPHAKEKS_ID),
            name: "AlphaKeks",
        })
        .await
        {
            Ok(_) | Err(CreateUserError::UserAlreadyExists) => {},
            Err(CreateUserError::Database(error)) => {
                return Err(error).context("failed to create AlphaKeks");
            },
        }
    }

    let owner_id = owner_id.unwrap_or(UserId::new(ALPHAKEKS_ID));
    let first_name = FirstName();
    let last_name = LastName();
    let word = Word();

    for _ in 0..count {
        let name = format!(
            "{} {}'s {}",
            first_name.fake::<String>(),
            last_name.fake::<String>(),
            word.fake::<String>()
        );
        let host = Faker.fake::<ServerHost>();
        let port = Faker.fake::<u16>();
        let server = NewServer { name: &name, host: &host, port, owner_id };

        match servers::approve(cx, server).await {
            Ok((id, access_key)) => info!(%id, %access_key, "created server"),
            Err(ApproveServerError::NameAlreadyTaken) => warn!(name, "name already taken"),
            Err(ApproveServerError::HostAndPortAlreadyTaken) => {
                warn!(%host, port, "host+port already taken")
            },
            Err(ApproveServerError::OwnerDoesNotExist) => anyhow::bail!("owner does not exist"),
            Err(ApproveServerError::Database(error)) => return Err(error.into()),
        }
    }

    Ok(())
}

async fn delete_servers(
    cx: &Context,
    owner_id: Option<UserId>,
    count: usize,
) -> anyhow::Result<()> {
    if let amount @ 1.. = servers::delete(cx, owner_id, count).await? {
        info!(amount, "deleted servers");
    } else if let Some(owner_id) = owner_id {
        anyhow::bail!("there are no servers owned by {owner_id} in the database");
    } else {
        anyhow::bail!("there are no servers in the database");
    }

    Ok(())
}

async fn create_records(
    cx: &Context,
    player_id: Option<PlayerId>,
    server_id: Option<ServerId>,
    filter_id: Option<CourseFilterId>,
    plugin_version: Option<&semver::Version>,
    count: usize,
) -> anyhow::Result<()> {
    let player_ids = match player_id {
        Some(id) => vec![id],
        None => players::get(cx, GetPlayersParams { limit: Limit::MAX, ..Default::default() })
            .await?
            .map(|player| player.id)
            .collect()
            .await?
            .into_inner(),
    };

    let server_ids = match server_id {
        Some(id) => vec![id],
        None => servers::get(cx, GetServersParams { limit: Limit::MAX, ..Default::default() })
            .await?
            .map(|server| server.id)
            .collect()
            .await?
            .into_inner(),
    };

    let filter_ids = match filter_id {
        Some(id) => vec![id],
        None => {
            filters::get(cx, GetCourseFiltersParams { approved_only: true, ..Default::default() })
                .flat_map(|filters| match filters {
                    Ok(filters) => {
                        Either::Left(stream::iter([filters.vanilla.id, filters.classic.id]).map(Ok))
                    },
                    Err(error) => Either::Right(stream::once(future::ready(error)).map(Err)),
                })
                .try_collect()
                .await?
        },
    };

    let plugin_version_id = match plugin_version {
        Some(version) => {
            plugin::get_version(cx, version)
                .await?
                .context("plugin version does not exist")?
                .id
        },
        None => {
            plugin::get_latest_version(cx)
                .await?
                .context("there are no plugin versions in the database")?
                .id
        },
    };

    for _ in 0..count {
        let mut rng = thread_rng();
        let record = NewRecord {
            player_id: player_ids
                .choose(&mut rng)
                .copied()
                .context("there are no players in the database")?,
            server_id: server_ids
                .choose(&mut rng)
                .copied()
                .context("there are no servers in the database")?,
            filter_id: filter_ids
                .choose(&mut rng)
                .copied()
                .context("there are no filters in the database")?,
            styles: Styles::none(),
            teleports: if rng.gen_range(0..100) > 33 {
                rng.r#gen()
            } else {
                0
            },
            time: Faker.fake(),
            plugin_version_id,
        };

        match records::submit(cx, record).await {
            Ok(SubmittedRecord { record_id: id, .. }) => info!(%id, "created record"),
            Err(SubmitRecordError::CalculatePoints(error)) => return Err(error.into()),
            Err(SubmitRecordError::Database(error)) => return Err(error.into()),
        }
    }

    Ok(())
}

async fn delete_records(
    cx: &Context,
    filter_id: Option<CourseFilterId>,
    starting_at: Option<RecordId>,
    count: usize,
) -> anyhow::Result<()> {
    if let amount @ 1.. = records::delete(cx, filter_id, starting_at, count).await? {
        info!(amount, "deleted records");
    } else {
        anyhow::bail!("there are no records in the database");
    }

    Ok(())
}

async fn create_maps(cx: &Context, mappers: Vec<PlayerId>, count: usize) -> anyhow::Result<()> {
    let mappers = if mappers.is_empty() {
        players::get(cx, GetPlayersParams { limit: Limit::MAX, ..Default::default() })
            .await?
            .map(|player| player.id)
            .collect::<Vec<_>>()
            .await?
            .into_inner()
            .into_boxed_slice()
    } else {
        mappers.into_boxed_slice()
    };

    if mappers.is_empty() {
        anyhow::bail!("there are no mappers in the database");
    }

    let mut rng = thread_rng();
    let gen_course_filter = |rng: &mut ThreadRng| {
        let nub_tier = Tier::try_from(rng.gen_range(1..=8)).unwrap();
        NewCourseFilter {
            nub_tier,
            pro_tier: Tier::try_from(rng.gen_range((nub_tier as u8)..=8)).unwrap(),
            state: CourseFilterState::Ranked,
            notes: if rng.r#gen() {
                Some(Paragraph(1..6).fake::<String>())
            } else {
                None
            },
        }
    };

    for _ in 0..count {
        let mappers = {
            let mapper_count = rng.gen_range(1..=cmp::min(10, mappers.len()));
            mappers
                .choose_multiple(&mut rng, mapper_count)
                .copied()
                .collect::<Vec<_>>()
                .into_boxed_slice()
        };

        let map = NewMap {
            workshop_id: WorkshopId::from_inner(Faker.fake::<u32>()),
            name: format!("kz_{}", Word().fake::<String>()),
            description: if rng.r#gen() {
                Some(Paragraph(1..6).fake::<String>())
            } else {
                None
            },
            state: MapState::Approved,
            vpk_checksum: Faker.fake(),
            mappers: mappers.clone(),
            courses: (0..rng.gen_range(1..=10))
                .map(|idx| NewCourse {
                    name: format!("Course {}", idx + 1),
                    description: if rng.r#gen() {
                        Some(Paragraph(1..6).fake::<String>())
                    } else {
                        None
                    },
                    mappers: {
                        let mapper_count = rng.gen_range(1..=mappers.len());
                        mappers
                            .choose_multiple(&mut rng, mapper_count)
                            .copied()
                            .collect::<Vec<_>>()
                            .into_boxed_slice()
                    },
                    filters: NewCourseFilters {
                        vanilla: gen_course_filter(&mut rng),
                        classic: gen_course_filter(&mut rng),
                    },
                })
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        };

        match maps::approve(cx, map).await {
            Ok(id) => info!(%id, "created map"),
            Err(ApproveMapError::Database(error)) => return Err(error.into()),
        }
    }

    Ok(())
}

async fn delete_maps(cx: &Context, starting_at: Option<MapId>, count: usize) -> anyhow::Result<()> {
    if let amount @ 1.. = maps::delete(cx, starting_at, count).await? {
        info!(amount, "deleted maps");
    } else {
        anyhow::bail!("there are no maps in the database");
    }

    Ok(())
}
