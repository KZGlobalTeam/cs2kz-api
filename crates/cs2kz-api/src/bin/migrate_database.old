#![feature(iter_array_chunks)]

use std::collections::HashMap;
use std::env;
use std::num::NonZero;
use std::time::Duration;

use anyhow::Context as _;
use cs2kz::database::{self, Database, DatabaseConnectionOptions, QueryBuilder};
use cs2kz::time::Timestamp;
use futures_util::TryStreamExt as _;
use tracing_subscriber::EnvFilter;
use url::Url;
use uuid::Uuid;

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new("migrate_database=trace,warn"))
        .init();

    let database_url = env::var("DATABASE_URL")
        .context("missing `DATABASE_URL` environment variable")?
        .parse::<Url>()
        .context("`DATABASE_URL` is not a valid URL")?;

    let database = Database::connect(DatabaseConnectionOptions {
        url: &database_url,
        min_connections: 1,
        max_connections: Some(const { NonZero::new(2).unwrap() }),
    })
    .await
    .context("failed to connect to database")?;

    database
        .in_transaction(migrate_database)
        .await
        .context("failed to migrate records")
}

async fn migrate_database(conn: &mut database::Connection) -> anyhow::Result<()> {
    let tables_to_copy_as_is = [
        "Announcements",
        "PluginVersions",
        "ModeChecksums",
        "StyleChecksums",
        "AccessKeys",
        "Users",
        "UserSessions",
        "Servers",
        "Players",
        "Maps",
        "Courses",
        "CourseFilters",
        "Mappers",
        "CourseMappers",
        "PointDistributionData",
        "Bans",
        "Unbans",
    ];

    for table in tables_to_copy_as_is {
        if table == "Players" {
            sqlx::query(
                "INSERT INTO `cs2kz`.`Players`
                 SELECT id, name, ip_address, preferences, 0.0, 0.0, first_joined_at, last_joined_at
                 FROM `cs2kz_old`.`Players`",
            )
            .execute(&mut *conn)
            .await
            .context("failed to copy table")?;
        } else {
            sqlx::query(&format! {
                "INSERT INTO `cs2kz`.`{table}`
                 SELECT * FROM `cs2kz_old`.`{table}`",
            })
            .execute(&mut *conn)
            .await
            .context("failed to copy tables")?;
        }
    }

    let uuid_cx = uuid::timestamp::context::ContextV7::new();

    let records = sqlx::query!("SELECT * FROM `cs2kz_old`.`Records` ORDER BY id ASC")
        .fetch(&mut *conn)
        .map_ok(|row| {
            let new_columns = (
                generate_record_uuid(&uuid_cx, row.submitted_at.into()),
                row.player_id,
                row.server_id,
                row.filter_id,
                row.styles,
                row.teleports,
                row.time,
                row.plugin_version_id,
            );

            (row.id, new_columns)
        })
        .try_collect::<HashMap<_, _>>()
        .await
        .context("failed to fetch records")?;

    let mut records_to_insert = records.values().array_chunks::<5000>();

    for chunk in records_to_insert.by_ref() {
        insert_records(&mut *conn, chunk).await?;
    }
    insert_records(&mut *conn, records_to_insert.into_remainder()).await?;

    let best_nub_records = sqlx::query!("SELECT * FROM `cs2kz_old`.BestNubRecords")
        .fetch_all(&mut *conn)
        .await?;

    for chunk in best_nub_records.chunks(5000) {
        let mut query = QueryBuilder::new("INSERT INTO BestNubRecords ");

        query.push_values(chunk, |mut query, record| {
            query.push_bind(record.filter_id);
            query.push_bind(record.player_id);
            query.push_bind(records[&record.record_id].0.as_bytes().to_vec());
            query.push_bind(record.points);
            query.push_bind(records[&record.record_id].6);
        });

        query.build().execute(&mut *conn).await?;
    }

    let best_pro_records = sqlx::query!("SELECT * FROM `cs2kz_old`.BestProRecords")
        .fetch_all(&mut *conn)
        .await?;

    for chunk in best_pro_records.chunks(5000) {
        let mut query = QueryBuilder::new("INSERT INTO BestProRecords ");

        query.push_values(chunk, |mut query, record| {
            query.push_bind(record.filter_id);
            query.push_bind(record.player_id);
            query.push_bind(records[&record.record_id].0.as_bytes().to_vec());
            query.push_bind(record.points);
            query.push_bind(records[&record.record_id].6);
        });

        query.build().execute(&mut *conn).await?;
    }

    Ok(())
}

async fn insert_records(
    conn: &mut database::Connection,
    records: impl IntoIterator<Item = &(Uuid, u64, u16, u16, u32, u32, f64, u16)>,
) -> anyhow::Result<()> {
    let mut query = QueryBuilder::new("INSERT INTO Records ");
    let mut was_empty = true;

    query.push_values(records, |mut query, record| {
        was_empty = false;

        query.push_bind(record.0.as_bytes().to_vec());
        query.push_bind(record.1);
        query.push_bind(record.2);
        query.push_bind(record.3);
        query.push_bind(record.4);
        query.push_bind(record.5);
        query.push_bind(record.6);
        query.push_bind(record.7);
    });

    if was_empty {
        return Ok(());
    }

    query.build().execute(&mut *conn).await?;

    Ok(())
}

fn generate_record_uuid(cx: &uuid::timestamp::context::ContextV7, submitted_at: Timestamp) -> Uuid {
    let unix_ts = Duration::from_millis(submitted_at.to_unix_ms());
    let uuid_ts = uuid::Timestamp::from_unix(cx, unix_ts.as_secs(), unix_ts.subsec_nanos());

    Uuid::new_v7(uuid_ts)
}

#[test]
fn generate_record_uuid_works() {
    let uuid_cx = uuid::timestamp::context::ContextV7::new();
    let ts = Timestamp::now();
    let uuid = generate_record_uuid(&uuid_cx, ts);

    assert_eq!(uuid.get_timestamp().unwrap().to_unix(), (
        Duration::from_millis(ts.to_unix_ms()).as_secs(),
        Duration::from_millis(ts.to_unix_ms()).subsec_nanos(),
    ));
}
