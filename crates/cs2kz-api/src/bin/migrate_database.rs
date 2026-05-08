use std::collections::HashMap;
use std::env;
use std::num::NonZero;

use anyhow::Context as _;
use cs2kz::database::{self, Database, DatabaseConnectionOptions, QueryBuilder};
use cs2kz::maps::CourseFilterId;
use cs2kz::records::RecordId;
use futures_util::TryStreamExt as _;
use tracing_subscriber::EnvFilter;
use url::Url;

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
    let record_ids = sqlx::query!(
        "SELECT
           id AS `id: RecordId`,
           filter_id AS `filter_id: CourseFilterId`,
           teleports,
           time
         FROM Records
         ORDER BY id ASC",
    )
    .fetch(&mut *conn)
    .try_fold(HashMap::<_, (Vec<_>, Vec<_>)>::new(), async |mut record_ids, row| {
        let (nub, pro) = record_ids.entry(row.filter_id).or_default();

        if nub.last().is_none_or(|&(_, time)| time > row.time) {
            nub.push((row.id, row.time));
        }

        if row.teleports == 0 && pro.last().is_none_or(|&(_, time)| time > row.time) {
            pro.push((row.id, row.time));
        }

        Ok(record_ids)
    })
    .await
    .context("failed to query records")?
    .into_values()
    .flat_map(|(nub, pro)| {
        let nub = nub.into_iter().map(|(record_id, _time)| record_id);
        let pro = pro.into_iter().map(|(record_id, _time)| record_id);
        std::iter::chain(nub, pro)
    })
    .collect::<Vec<_>>();

    let mut query = QueryBuilder::new("INSERT INTO WorldRecords ");

    for chunk in record_ids.chunks(5000) {
        query.reset();
        query.push_values(chunk, |mut query, &record_id| {
            query.push_bind(record_id);
        });
        query.push(" ON DUPLICATE KEY UPDATE id=VALUES(id) ");
        query
            .build()
            .persistent(false)
            .execute(&mut *conn)
            .await
            .context("failed to insert record ids")?;
    }

    Ok(())
}
