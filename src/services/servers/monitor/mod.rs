#![allow(unused_imports, missing_docs, clippy::missing_docs_in_private_items)]

use std::future::{self, Future};
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::{Acquire, Relaxed};
use std::time::Duration;

use chrono::{DateTime, Utc};
use cs2kz::SteamID;
use futures::TryStreamExt;
use sqlx::pool::PoolConnection;
use sqlx::{MySql, Pool};
use tap::{Pipe, Tap};
use tokio::task;
use tokio::time::MissedTickBehavior;

use crate::services::servers::ServerID;
use crate::time::DurationExt;

mod error;
pub use error::Error;

const VERY_DEAD_THRESHOLD: chrono::Duration = chrono::Duration::hours(6);
const DEAD_THRESHOLD: chrono::Duration = chrono::Duration::minutes(15);

/// Whether the monitor is currently running.
static RUNNING: AtomicBool = AtomicBool::new(false);

pub struct Config
{
	interval: Duration,
}

pub fn spawn(config: Config, database: Pool<MySql>) -> bool
{
	// Ensure we only spawn one instance.
	if let Err(_) = RUNNING.compare_exchange(false, true, Acquire, Relaxed) {
		return false;
	}

	task::spawn(async move {
		let mut interval = tokio::time::interval(config.interval).tap_mut(|interval| {
			interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
		});

		loop {
			interval.tick().await;

			if let Err(error) = run(database.clone()).await {
				tracing::error! {
					target: "cs2kz_api::runtime::errors",
					?error,
					"server monitor execution failed",
				};
			}
		}
	});

	true
}

async fn run(pool: Pool<MySql>) -> Result<(), Error>
{
	let now = Utc::now();

	let mut servers = sqlx::query_as! {
		Server,
		r"
		SELECT
		  id `id: ServerID`,
		  name,
		  owner_id `owner_id: SteamID`,
		  status `status: Status`,
		  last_seen_on `last_seen_on: DateTime<Utc>`
		FROM
		  Servers
		",
	}
	.fetch(&pool)
	.map_ok(|server| {
		let elapsed = now.signed_duration_since(server.last_seen_on);
		let new_status = if elapsed >= VERY_DEAD_THRESHOLD {
			Status::VeryDead
		} else if elapsed >= DEAD_THRESHOLD {
			Status::Dead
		} else {
			Status::Alive
		};

		(server, new_status)
	});

	while let Some((server, mut new_status)) = servers.try_next().await? {
		match (server.status, new_status) {
			// Nothing changed.
			(Status::Alive, Status::Alive)
			| (Status::Dead, Status::Dead)
			| (Status::VeryDead, Status::VeryDead) => {
				continue;
			}

			// Server died / came back to life, just update the status.
			(Status::Alive, Status::Dead | Status::VeryDead)
			| (Status::Dead | Status::VeryDead, Status::Alive) => {
				// This can only happen if the server was 'alive', and then the API
				// shut down. It then had a downtime of 6h+, and now the server
				// hasn't checked in yet! We would now wrongly assume that it
				// hasn't checked in for 6h+, even though we weren't even running
				// ourselves! Let's just treat this one as 'dead' for now.
				if new_status == Status::VeryDead {
					new_status = Status::Dead;
				}

				let mut conn = pool.acquire().await?;

				task::spawn(report_error(async move {
					update_status(&server, new_status, &mut conn).await
				}));
			}

			// Server hasn't checked in for too long, notify the owner!
			(Status::Dead, Status::VeryDead) => {
				let mut conn = pool.acquire().await?;

				task::spawn(report_error(async move {
					update_status(&server, new_status, &mut conn).await?;
					notify_server_owner(&server, &mut conn).await
				}));
			}

			// We never change statuses like that.
			(Status::VeryDead, Status::Dead) => unreachable!(),
		}
	}

	Ok(())
}

async fn report_error<F, E>(f: F)
where
	F: Future<Output = Result<(), E>> + Send + 'static,
	E: std::error::Error + Send + 'static,
{
	if let Err(error) = f.await {
		let error = &error as &dyn std::error::Error;
		tracing::error!(target: "cs2kz_api::runtime::errors", error);
	}
}

async fn update_status(
	server: &Server,
	status: Status,
	conn: &mut PoolConnection<MySql>,
) -> Result<(), Error>
{
	sqlx::query!("UPDATE Servers SET status = ? WHERE id = ?", status, server.id)
		.execute(conn.as_mut())
		.await?;

	Ok(())
}

async fn notify_server_owner(
	_server: &Server,
	_conn: &mut PoolConnection<MySql>,
) -> Result<(), Error>
{
	// TODO

	Ok(())
}

#[derive(Debug)]
struct Server
{
	id: ServerID,
	name: String,
	owner_id: SteamID,
	last_seen_on: DateTime<Utc>,
	status: Status,
}

#[repr(i8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
enum Status
{
	Alive = 1,
	Dead = 0,
	VeryDead = -1,
}

enum Action
{
	NotifyServerOwner,
}
