//! A service for managing records.

use std::fmt;

use axum::extract::FromRef;
use sqlx::{MySql, Pool, QueryBuilder, Row, Transaction};
use tap::Tap;

use crate::database::TransactionExt;
use crate::services::AuthService;

pub(crate) mod http;

mod error;
pub use error::{Error, Result};

pub(crate) mod models;
pub use models::{
	FetchRecordRequest,
	FetchRecordResponse,
	FetchRecordsRequest,
	FetchRecordsResponse,
	FetchReplayRequest,
	FetchReplayResponse,
	RecordID,
	RecordStatus,
	SubmitRecordRequest,
	SubmitRecordResponse,
	UpdateRecordAction,
	UpdateRecordRequest,
	UpdateRecordResponse,
};

/// A service for managing records.
#[derive(Clone, FromRef)]
#[allow(clippy::missing_docs_in_private_items)]
pub struct RecordService
{
	database: Pool<MySql>,
	auth_svc: AuthService,
}

impl fmt::Debug for RecordService
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		f.debug_struct("RecordService").finish_non_exhaustive()
	}
}

impl RecordService
{
	/// Create a new [`RecordService`].
	#[tracing::instrument]
	pub fn new(database: Pool<MySql>, auth_svc: AuthService) -> Self
	{
		Self { database, auth_svc }
	}

	/// Fetch a record by its ID.
	#[tracing::instrument(level = "debug", err(Debug, level = "debug"))]
	pub async fn fetch_record(&self, req: FetchRecordRequest)
	-> Result<Option<FetchRecordResponse>>
	{
		let res = sqlx::query_as(
			r"
			SELECT
			  r.id,
			  f.mode,
			  r.styles,
			  r.teleports,
			  r.time,
			  c.id course_id,
			  c.name course_name,
			  m.id course_map_id,
			  m.name course_map_name,
			  f.tier course_tier,
			  f.ranked_status course_ranked_status,
			  p.id player_id,
			  p.name player_name,
			  s.id server_id,
			  s.name server_name,
			  r.bhops,
			  r.perfs,
			  r.perfect_perfs,
			  r.created_on
			FROM
			  Records r
			  JOIN CourseFilters f ON f.id = r.filter_id
			  JOIN Courses c ON c.id = f.course_id
			  JOIN Maps m ON m.id = c.map_id
			  JOIN Players p ON p.id = r.player_id
			  JOIN Servers s ON s.id = r.server_id
			WHERE
			  r.id = ?
			",
		)
		.bind(req.record_id)
		.fetch_optional(&self.database)
		.await?;

		Ok(res)
	}

	/// Fetch potentially many records.
	#[tracing::instrument(level = "debug", err(Debug, level = "debug"))]
	pub async fn fetch_records(&self, req: FetchRecordsRequest) -> Result<FetchRecordsResponse>
	{
		let mut txn = self.database.begin().await?;

		let (min_tp, max_tp) = match req.has_teleports {
			None => (None, None),
			Some(true) => (Some(1), None),
			Some(false) => (None, Some(0)),
		};

		let course_id = match req.course {
			None => None,
			Some(ident) => ident.resolve_id(txn.as_mut()).await?,
		};

		let map_id = match req.map {
			None => None,
			Some(ident) => ident.resolve_id(txn.as_mut()).await?,
		};

		let player_id = match req.player {
			None => None,
			Some(ident) => ident.resolve_id(txn.as_mut()).await?,
		};

		let server_id = match req.server {
			None => None,
			Some(ident) => ident.resolve_id(txn.as_mut()).await?,
		};

		// TODO:
		// - handle `req.top`
		let records = sqlx::query_as(
			r"
			SELECT SQL_CALC_FOUND_ROWS
			  r.id,
			  f.mode,
			  r.styles,
			  r.teleports,
			  r.time,
			  c.id course_id,
			  c.name course_name,
			  m.id course_map_id,
			  m.name course_map_name,
			  f.tier course_tier,
			  f.ranked_status course_ranked_status,
			  p.id player_id,
			  p.name player_name,
			  s.id server_id,
			  s.name server_name,
			  r.bhops bhops_total,
			  r.perfs bhops_perfs,
			  r.perfect_perfs bhops_perfect_perfs,
			  r.created_on
			FROM
			  Records r
			  JOIN CourseFilters f ON f.id = r.filter_id
			  JOIN Courses c ON c.id = f.course_id
			  JOIN Maps m ON m.id = c.map_id
			  JOIN Players p ON p.id = r.player_id
			  JOIN Servers s ON s.id = r.server_id
			WHERE
			  f.mode = COALESCE(?, f.mode)
			  AND r.styles = COALESCE(?, r.styles)
			  AND (
			    r.teleports BETWEEN COALESCE(?, 0) AND COALESCE(?, (1 << 31))
			  )
			  AND c.id = COALESCE(?, c.id)
			  AND m.id = COALESCE(?, m.id)
			  AND p.id = COALESCE(?, p.id)
			  AND s.id = COALESCE(?, s.id)
			  AND r.created_on > COALESCE(?, '1970-01-01 00:00:01')
			  AND r.created_on < COALESCE(?, '2038-01-19 03:14:07')
			LIMIT
			  ? OFFSET ?
			",
		)
		.bind(req.mode)
		.bind(req.styles)
		.bind(min_tp)
		.bind(max_tp)
		.bind(course_id)
		.bind(map_id)
		.bind(player_id)
		.bind(server_id)
		.bind(req.created_after)
		.bind(req.created_before)
		.bind(*req.limit)
		.bind(*req.offset)
		.fetch_all(txn.as_mut())
		.await?;

		let total = txn.total_rows().await?;

		txn.commit().await?;

		Ok(FetchRecordsResponse { records, total })
	}

	/// Fetch the replay for a record.
	#[tracing::instrument(level = "debug", err(Debug, level = "debug"))]
	pub async fn fetch_replay(&self, req: FetchReplayRequest) -> Result<FetchReplayResponse>
	{
		Ok(FetchReplayResponse { _priv: () })
	}

	/// Submit a new record.
	#[tracing::instrument(level = "debug", err(Debug, level = "debug"))]
	pub async fn submit_record(&self, req: SubmitRecordRequest) -> Result<SubmitRecordResponse>
	{
		let record_id = sqlx::query! {
			r"
			INSERT INTO
			  Records (
			    filter_id,
			    styles,
			    teleports,
			    time,
			    player_id,
			    server_id,
			    bhops,
			    perfs,
			    perfect_perfs,
			    plugin_version_id
			  )
			VALUES
			  (
			    (
			      SELECT
				id
			      FROM
				CourseFilters
			      WHERE
				course_id = ?
				AND mode = ?
				AND teleports = ?
			      LIMIT
				1
			    ), ?, ?, ?, ?, ?, ?, ?, ?, ?
			  )
			RETURNING id
			",
			req.course_id,
			req.mode,
			req.teleports > 0,
			req.styles,
			req.teleports,
			req.time,
			req.player_id,
			req.server_id,
			req.bhop_stats.total,
			req.bhop_stats.perfs,
			req.bhop_stats.perfect_perfs,
			req.plugin_version_id,
		}
		.fetch_one(&self.database)
		.await
		.and_then(|row| row.try_get(0))?;

		Ok(SubmitRecordResponse { record_id })
	}

	/// Update an existing record.
	#[tracing::instrument(level = "debug", err(Debug, level = "debug"))]
	pub async fn update_record(&self, req: UpdateRecordRequest) -> Result<UpdateRecordResponse>
	{
		let mut txn = self.database.begin().await?;

		match req.action {
			UpdateRecordAction::ChangeStatus { new_status } => {
				move_record(req.record_id, new_status, &mut txn).await?;
			}
		}

		txn.commit().await?;

		Ok(UpdateRecordResponse { _priv: () })
	}
}

#[tracing::instrument(level = "trace", err(Debug, level = "debug"))]
async fn move_record(
	record_id: RecordID,
	to: RecordStatus,
	txn: &mut Transaction<'_, MySql>,
) -> Result<()>
{
	let counts = sqlx::query! {
		r"
		SELECT
		  count(r.id) cnt_normal,
		  count(sr.id) cnt_sus,
		  count(sr.id) cnt_cheated,
		  count(wr.id) cnt_wiped
		FROM
		  Records r
		  JOIN SuspiciousRecords sr ON sr.id = ?
		  JOIN CheatedRecords cr ON cr.id = ?
		  JOIN WipedRecords wr ON wr.id = ?
		WHERE
		  r.id = ?
		",
		record_id,
		record_id,
		record_id,
		record_id,
	}
	.fetch_one(txn.as_mut())
	.await
	.map(|row| (row.cnt_normal, row.cnt_sus, row.cnt_cheated, row.cnt_wiped))?;

	let from = match counts {
		(1, 0, 0, 0) => RecordStatus::Default,
		(0, 1, 0, 0) => RecordStatus::Suspicious,
		(0, 0, 1, 0) => RecordStatus::Cheated,
		(0, 0, 0, 1) => RecordStatus::Wiped,

		(0, 0, 0, 0) => {
			return Err(Error::RecordDoesNotExist);
		}

		_ => panic!("duplicate record? {counts:?}"),
	};

	if from == to {
		return Err(Error::WouldNotMove);
	}

	let from = from.table_name();
	let to = to.table_name();

	let copy_result = QueryBuilder::new("INSERT INTO ")
		.tap_mut(|query| {
			query.push(to).push("SELECT * FROM ");
			query.push(from).push(" WHERE id = ").push_bind(record_id);
		})
		.build()
		.execute(txn.as_mut())
		.await?;

	match copy_result.rows_affected() {
		1 => { /* great! */ }
		n => panic!("did not copy exactly 1 record, but {n}"),
	}

	let delete_result = QueryBuilder::new("DELETE FROM ")
		.tap_mut(|query| {
			query.push(from).push(" WHERE id = ").push_bind(record_id);
		})
		.build()
		.execute(txn.as_mut())
		.await?;

	match delete_result.rows_affected() {
		1 => { /* great! */ }
		n => panic!("did not delete exactly 1 record, but {n}"),
	}

	tracing::info!(from, to, "moved record");

	Ok(())
}
