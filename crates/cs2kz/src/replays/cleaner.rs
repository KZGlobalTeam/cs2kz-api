use std::time::Duration;

use tokio::time::{interval, sleep};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::Context;
use crate::maps::courses::filters::CourseFilterState;
use crate::records::RecordId;
use crate::time::Timestamp;

// AWS limits deletion requests to at most 1k objects per request
const MAX_OBJECTS_PER_REQUEST: usize = 1000;

pub async fn periodically_clean(cx: Context, cancellation_token: CancellationToken) {
    let Some(ref storage_config) = cx.config().replay_storage else {
        warn!("no replay storage configured");
        return;
    };

    let s3_client = cx.s3_client();

    let mut interval = interval(Duration::from_hours(24));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    while cancellation_token
        .run_until_cancelled(interval.tick())
        .await
        .is_some()
    {
        let now = Timestamp::now();
        let last_cleaned_at = match sqlx::query_scalar!(
            "SELECT last_cleaned_at AS `last_cleaned_at: Timestamp`
             FROM ReplayCleanerData"
        )
        .fetch_one(cx.database().as_ref())
        .await
        {
            Ok(ts) => ts,
            Err(error) => {
                error!(%error, "failed to query ReplayCleanerData table");
                if cancellation_token
                    .run_until_cancelled(sleep(Duration::from_secs(10)))
                    .await
                    .is_none()
                {
                    break;
                }
                continue;
            },
        };

        let record_ids = match sqlx::query_scalar!(
            "WITH NubRecordsWithRanks AS (
               SELECT
                 record_id AS id,
                 ROW_NUMBER() OVER (
                   PARTITION BY filter_id
                   ORDER BY time ASC, id ASC
                 ) AS rank
               FROM
                 BestNubRecords
             ),
             ProRecordsWithRanks AS (
               SELECT
                 record_id AS id,
                 ROW_NUMBER() OVER (
                   PARTITION BY filter_id
                   ORDER BY time ASC, id ASC
                 ) AS rank
               FROM
                 BestProRecords
             )
             SELECT r.id AS `id: RecordId`
             FROM Records AS r
             INNER JOIN CourseFilters AS cf ON cf.id = r.filter_id
             LEFT OUTER JOIN NubRecordsWithRanks AS bnr ON bnr.id = r.id
             LEFT OUTER JOIN NubRecordsWithRanks AS bpr ON bpr.id = r.id
             LEFT OUTER JOIN WorldRecords AS wr ON wr.id = r.id

             -- too old?
             WHERE r.id < ?
             AND (? OR r.id < ?)

             -- not T8?
             AND (cf.nub_tier < 8 AND cf.pro_tier < 8)

             AND (
               -- not ranked?
               cf.state != ?

               -- not top 10?
               AND (bnr.id IS NULL
                OR bnr.rank > 10)
               AND (bpr.id IS NULL
                OR bpr.rank > 10)
             )

             -- not a WR?
             AND wr.id IS NULL",
            RecordId::from_uuid(Uuid::new_v7(
                (Timestamp::now() + time::Duration::hours(-24)).into()
            )),
            last_cleaned_at.is_none(),
            last_cleaned_at.map(|ts| RecordId::from_uuid(Uuid::new_v7(ts.into()))),
            CourseFilterState::Ranked,
        )
        .fetch_all(cx.database().as_ref())
        .await
        {
            Ok(results) => results,
            Err(error) => {
                error!(%error, "failed to query records");
                if cancellation_token
                    .run_until_cancelled(sleep(Duration::from_secs(10)))
                    .await
                    .is_none()
                {
                    break;
                }
                continue;
            },
        };

        for ids in record_ids.chunks(MAX_OBJECTS_PER_REQUEST) {
            let delete = ids
                .iter()
                .fold(aws_sdk_s3::types::Delete::builder(), |delete, id| {
                    let object = aws_sdk_s3::types::ObjectIdentifier::builder()
                        .key(id.to_string())
                        .build()
                        .unwrap();

                    delete.objects(object)
                })
                .quiet(true)
                .build()
                .unwrap();

            if let Err(error) = s3_client
                .delete_objects()
                .bucket(&storage_config.bucket_name)
                .delete(delete)
                .send()
                .await
            {
                error!(%error, "failed to delete objects");
            }
        }

        if let Err(error) = cx
            .database()
            .in_transaction(async |conn| -> Result<_, crate::database::Error> {
                sqlx::query!("DELETE FROM ReplayCleanerData")
                    .execute(&mut *conn)
                    .await?;

                sqlx::query!("INSERT INTO ReplayCleanerData VALUES (?)", now)
                    .execute(&mut *conn)
                    .await?;

                Ok(())
            })
            .await
        {
            error!(%error, "failed to update ReplayCleanerData");
        }
    }
}
