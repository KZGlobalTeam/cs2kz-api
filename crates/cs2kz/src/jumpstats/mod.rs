use std::num::NonZero;

use futures_util::{Stream, TryStreamExt};

use crate::mode::Mode;
use crate::pagination::{Limit, Offset, Paginated};
use crate::players::{PlayerId, PlayerInfo};
use crate::servers::{ServerId, ServerInfo};
use crate::styles::Styles;
use crate::time::{Seconds, Timestamp};
use crate::{Context, database};

mod jump_type;
pub use jump_type::JumpType;

define_id_type! {
    /// A unique identifier for jumpstats.
    #[derive(sqlx::Type)]
    #[sqlx(transparent)]
    pub struct JumpstatId(NonZero<u32>);
}

#[derive(Debug)]
pub struct Jumpstat {
    pub id: JumpstatId,
    pub player: PlayerInfo,
    pub server: ServerInfo,
    pub mode: Mode,
    pub styles: Styles,
    pub jump_type: JumpType,
    pub time: Seconds,
    pub strafes: u8,
    pub distance: f32,
    pub sync: f32,
    pub pre: f32,
    pub max: f32,
    pub overlap: f32,
    pub bad_angles: f32,
    pub dead_air: f32,
    pub height: f32,
    pub airpath: f32,
    pub deviation: f32,
    pub average_width: f32,
    pub submitted_at: Timestamp,
}

#[derive(Debug)]
pub struct GetJumpstatsParams {
    pub limit: Limit<1000, 100>,
    pub offset: Offset,
}

#[derive(Debug, Display, Error, From)]
#[display("failed to get jumpstats")]
#[from(forward)]
pub struct GetJumpstatsError(database::Error);

#[tracing::instrument(skip(cx), ret(level = "debug"), err(level = "debug"))]
pub async fn get(
    cx: &Context,
    GetJumpstatsParams { limit, offset }: GetJumpstatsParams,
) -> Result<Paginated<impl Stream<Item = Result<Jumpstat, GetJumpstatsError>>>, GetJumpstatsError> {
    let total = database::count!(cx.database().as_ref(), "Jumps").await?;
    let jumpstats = self::macros::select!("LIMIT ? OFFSET ?", limit.value(), offset.value())
        .fetch(cx.database().as_ref())
        .map_ok(|row| self::macros::parse_row!(row))
        .map_err(GetJumpstatsError::from);

    Ok(Paginated::new(total, jumpstats))
}

#[tracing::instrument(skip(cx), err(level = "debug"))]
pub async fn get_by_id(
    cx: &Context,
    jumpstat_id: JumpstatId,
) -> Result<Option<Jumpstat>, GetJumpstatsError> {
    self::macros::select!("WHERE j.id = ?", jumpstat_id)
        .fetch_optional(cx.database().as_ref())
        .await
        .map(|row| row.map(|row| self::macros::parse_row!(row)))
        .map_err(GetJumpstatsError::from)
}

#[tracing::instrument(skip(cx), err(level = "debug"))]
pub async fn get_replay(
    cx: &Context,
    jumpstat_id: JumpstatId,
) -> Result<Option<Vec<u8>>, GetJumpstatsError> {
    sqlx::query_scalar!("SELECT data FROM JumpReplays WHERE jump_id = ?", jumpstat_id)
        .fetch_optional(cx.database().as_ref())
        .await
        .map_err(GetJumpstatsError::from)
}

mod macros {
    macro_rules! select {
        ( $($extra:tt)* ) => {
            sqlx::query!(
                "SELECT
                   j.id AS `id: JumpstatId`,
                   p.id AS `player_id: PlayerId`,
                   p.name AS player_name,
                   s.id AS `server_id: ServerId`,
                   s.name AS server_name,
                   j.mode AS `mode: Mode`,
                   j.styles AS `styles: Styles`,
                   j.type AS `jump_type: JumpType`,
                   j.time AS `time: Seconds`,
                   j.strafes,
                   j.distance,
                   j.sync,
                   j.pre,
                   j.max,
                   j.overlap,
                   j.bad_angles,
                   j.dead_air,
                   j.height,
                   j.airpath,
                   j.deviation,
                   j.average_width,
                   j.submitted_at
                 FROM Jumps AS j
                 JOIN Players AS p ON p.id = j.player_id
                 JOIN Servers AS s ON s.id = j.server_id "
                + $($extra)*
            )
        };
    }

    macro_rules! parse_row {
        ($row:expr) => {
            Jumpstat {
                id: $row.id,
                player: PlayerInfo { id: $row.player_id, name: $row.player_name },
                server: ServerInfo { id: $row.server_id, name: $row.server_name },
                mode: $row.mode,
                styles: $row.styles,
                jump_type: $row.jump_type,
                time: $row.time,
                strafes: $row.strafes,
                distance: $row.distance,
                sync: $row.sync,
                pre: $row.pre,
                max: $row.max,
                overlap: $row.overlap,
                bad_angles: $row.bad_angles,
                dead_air: $row.dead_air,
                height: $row.height,
                airpath: $row.airpath,
                deviation: $row.deviation,
                average_width: $row.average_width,
                submitted_at: $row.submitted_at.into(),
            }
        };
    }

    pub(super) use {parse_row, select};
}
