use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::num::NonZero;
use std::{array, iter};

use futures_util::{Stream, StreamExt, TryFutureExt, TryStreamExt};
use sqlx::Row;

use self::courses::filters::{CourseFilterState, Tier};
use self::stream::{GetMapsStream, RawCourse, RawCourseFilters, RawMap};
use crate::Context;
use crate::database::{self, QueryBuilder};
use crate::events::{self, Event};
use crate::mode::Mode;
use crate::pagination::{Limit, Offset, Paginated};
use crate::players::{PlayerId, PlayerInfo};
use crate::steam::WorkshopId;
use crate::time::Timestamp;

mod state;
pub use state::MapState;

mod checksum;
pub use checksum::MapChecksum;

mod stream;

pub mod courses;
pub use courses::filters::CourseFilterId;
pub use courses::{CourseId, CourseInfo};

define_id_type! {
    /// A unique identifier for CS2KZ maps.
    #[derive(sqlx::Type)]
    #[sqlx(transparent)]
    pub struct MapId(NonZero<u16>);
}

#[derive(Debug, serde::Serialize)]
pub struct Map {
    pub id: MapId,
    pub workshop_id: WorkshopId,
    pub name: String,
    pub description: Option<String>,
    pub state: MapState,
    pub vpk_checksum: MapChecksum,
    pub mappers: Vec<PlayerInfo>,
    pub courses: Vec<Course>,
    pub approved_at: Timestamp,
}

impl Map {
    pub fn find_course_by_name(&self, course_name: &str) -> Option<&Course> {
        let lowercase = course_name.to_lowercase();

        self.courses
            .iter()
            .find(|course| course.name.to_lowercase().contains(&lowercase))
    }
}

#[derive(Debug, serde::Serialize)]
pub struct MapInfo {
    pub id: MapId,
    pub name: String,
}

#[derive(Debug, serde::Serialize)]
pub struct Course {
    pub id: CourseId,
    pub name: String,
    pub description: Option<String>,
    pub mappers: Vec<PlayerInfo>,
    pub filters: CourseFilters,
}

#[derive(Debug, serde::Serialize)]
pub struct CourseFilters {
    pub vanilla: CourseFilter,
    pub classic: CourseFilter,
}

#[derive(Debug, serde::Serialize)]
pub struct CourseFilter {
    pub id: CourseFilterId,
    pub nub_tier: Tier,
    pub pro_tier: Tier,
    pub state: CourseFilterState,
    pub notes: Option<String>,
}

#[derive(Debug)]
pub struct GetMapsParams<'a> {
    pub workshop_id: Option<WorkshopId>,
    pub name: Option<&'a str>,
    pub state: Option<MapState>,
    pub limit: Limit<1000, 100>,
    pub offset: Offset,
}

#[derive(Debug)]
pub struct NewMap {
    pub workshop_id: WorkshopId,
    pub name: String,
    pub description: Option<String>,
    pub state: MapState,
    pub vpk_checksum: MapChecksum,
    pub mappers: Box<[PlayerId]>,
    pub courses: Box<[NewCourse]>,
}

#[derive(Debug)]
pub struct NewCourse {
    pub name: String,
    pub description: Option<String>,
    pub mappers: Box<[PlayerId]>,
    pub filters: NewCourseFilters,
}

#[derive(Debug, Clone)]
pub struct NewCourseFilters {
    pub vanilla: NewCourseFilter,
    pub classic: NewCourseFilter,
}

#[derive(Debug, Clone)]
pub struct NewCourseFilter {
    pub nub_tier: Tier,
    pub pro_tier: Tier,
    pub state: CourseFilterState,
    pub notes: Option<String>,
}

#[derive(Debug)]
pub struct MapUpdate<'a> {
    pub id: MapId,
    pub workshop_id: Option<WorkshopId>,
    pub name: Option<&'a str>,
    pub description: Option<&'a str>,
    pub state: Option<MapState>,
    pub vpk_checksum: Option<MapChecksum>,
    pub added_mappers: &'a [PlayerId],
    pub deleted_mappers: &'a [PlayerId],
    pub course_updates: Vec<CourseUpdate<'a>>,
}

#[derive(Debug)]
pub struct CourseUpdate<'a> {
    pub idx: usize,
    pub name: Option<&'a str>,
    pub description: Option<&'a str>,
    pub added_mappers: &'a [PlayerId],
    pub deleted_mappers: &'a [PlayerId],
    pub filter_updates: FilterUpdates<'a>,
}

#[derive(Debug)]
pub struct FilterUpdates<'a> {
    pub vanilla: Option<FilterUpdate<'a>>,
    pub classic: Option<FilterUpdate<'a>>,
}

#[derive(Debug)]
pub struct FilterUpdate<'a> {
    pub nub_tier: Option<Tier>,
    pub pro_tier: Option<Tier>,
    pub state: Option<CourseFilterState>,
    pub notes: Option<&'a str>,
}

#[derive(Debug, Display, Error, From)]
#[display("failed to get maps")]
#[from(forward)]
pub struct GetMapsError(database::Error);

#[derive(Debug, Display, Error, From)]
pub enum ApproveMapError {
    #[display("{_0}")]
    #[from(forward)]
    Database(database::Error),
}

#[derive(Debug, Display, Error, From)]
pub enum UpdateMapError {
    #[display("maps and courses must always have at least one mapper")]
    MustHaveMappers,

    #[display("map does not have a course #{idx}")]
    #[error(ignore)]
    #[from(ignore)]
    InvalidCourseIndex { idx: usize },

    #[display("{_0}")]
    #[from(forward)]
    Database(database::Error),
}

#[tracing::instrument(skip(cx), ret(level = "debug"), err(level = "debug"))]
pub async fn get(
    cx: &Context,
    GetMapsParams { workshop_id, name, state, limit, offset }: GetMapsParams<'_>,
) -> Result<Paginated<impl Stream<Item = Result<Map, GetMapsError>>>, GetMapsError> {
    let total = database::count!(cx.database().as_ref(), "Maps").await?;
    let stream = self::macros::select!(
        cx.database().as_ref(),
        "WHERE m.workshop_id = COALESCE(?, m.workshop_id)
         AND m.name LIKE COALESCE(?, m.name)
         AND m.state = COALESCE(?, m.state)
         LIMIT ?
         OFFSET ?",
        workshop_id,
        name.map(|name| format!("%{name}%")),
        state,
        limit.value(),
        offset.value(),
    )
    .skip(offset.value().unsigned_abs() as usize)
    .take(limit.value() as usize);

    Ok(Paginated::new(total, stream))
}

#[tracing::instrument(skip(cx), err(level = "debug"))]
pub async fn get_by_id(cx: &Context, map_id: MapId) -> Result<Option<Map>, GetMapsError> {
    self::macros::select!(cx.database().as_ref(), "WHERE m.id = ?", map_id)
        .try_next()
        .map_err(GetMapsError::from)
        .await
}

#[tracing::instrument(skip(cx))]
pub fn get_by_name(cx: &Context, map_name: &str) -> impl Stream<Item = Result<Map, GetMapsError>> {
    self::macros::select!(cx.database().as_ref(), "WHERE m.name LIKE ?", format!("%{map_name}%"))
        .map_err(GetMapsError::from)
}

#[tracing::instrument(skip(cx), err(level = "debug"))]
pub async fn get_course_id_by_name(
    cx: &Context,
    course_name: &str,
) -> Result<Option<CourseId>, GetMapsError> {
    sqlx::query_scalar!(
        "SELECT id AS `id: CourseId`
         FROM Courses
         WHERE name LIKE ?",
        format!("%{course_name}%"),
    )
    .fetch_optional(cx.database().as_ref())
    .await
    .map_err(GetMapsError::from)
}

#[tracing::instrument(skip(cx), err(level = "debug"))]
pub async fn approve(
    cx: &Context,
    NewMap {
        workshop_id,
        name,
        description,
        state,
        vpk_checksum,
        mappers,
        courses,
    }: NewMap,
) -> Result<MapId, ApproveMapError> {
    cx.database_transaction(async move |conn| {
        match invalidate_old_map_rows(&mut *conn, &name).await? {
            0 => info!("approving new map '{name}'"),
            1 => info!("invalidated old version of '{name}'"),
            amount => warn!(amount, "invalidated multiple old versions of '{name}'"),
        }

        let map_id =
            insert_map(&mut *conn, workshop_id, &name, description.as_deref(), state, vpk_checksum)
                .await?;

        insert_mappers(&mut *conn, map_id, &mappers).await?;
        insert_courses(&mut *conn, map_id, &courses).await?;

        events::dispatch(Event::NewMap {
            workshop_id,
            name,
            description,
            state,
            vpk_checksum,
            mappers,
            courses,
        });

        Ok(map_id)
    })
    .await
}

#[tracing::instrument(skip(cx), err(level = "debug"))]
pub async fn update(
    cx: &Context,
    MapUpdate {
        id,
        workshop_id,
        name,
        description,
        state,
        vpk_checksum,
        added_mappers,
        deleted_mappers,
        course_updates,
    }: MapUpdate<'_>,
) -> Result<bool, UpdateMapError> {
    cx.database_transaction(async move |conn| {
        let updated = sqlx::query!(
            "UPDATE Maps
             SET workshop_id = COALESCE(?, workshop_id),
                 name = COALESCE(?, name),
                 description = COALESCE(?, description),
                 state = COALESCE(?, state),
                 vpk_checksum = COALESCE(?, vpk_checksum)
             WHERE id = ?",
            workshop_id,
            name,
            description,
            state,
            vpk_checksum,
            id,
        )
        .execute(&mut *conn)
        .await
        .map(|result| result.rows_affected() > 0)?;

        if !updated {
            return Ok(false);
        }

        insert_mappers(&mut *conn, id, added_mappers).await?;
        delete_mappers(&mut *conn, id, deleted_mappers).await?;

        let mapper_count = database::count!(&mut *conn, "Mappers").await?;

        if mapper_count == 0 {
            return Err(UpdateMapError::MustHaveMappers);
        }

        if course_updates.is_empty() {
            return Ok(true);
        }

        let course_ids =
            sqlx::query_scalar!("SELECT id AS `id: CourseId` FROM Courses WHERE map_id = ?", id)
                .fetch(&mut *conn)
                .zip(futures_util::stream::iter(1..))
                .map(|(row, idx)| row.map(|row| (idx, row)))
                .try_collect::<HashMap<_, _>>()
                .await?;

        for course_update in course_updates {
            let course_id = course_ids
                .get(&course_update.idx)
                .copied()
                .ok_or(UpdateMapError::InvalidCourseIndex { idx: course_update.idx })?;

            sqlx::query!(
                "UPDATE Courses
                 SET name = COALESCE(?, name),
                     description = COALESCE(?, description)
                 WHERE id = ?",
                course_update.name,
                course_update.description,
                course_id,
            )
            .execute(&mut *conn)
            .await?;

            if !course_update.added_mappers.is_empty() {
                insert_course_mappers(
                    &mut *conn,
                    iter::zip(iter::repeat(course_id), course_update.added_mappers.iter().copied()),
                )
                .await?;
            }

            delete_course_mappers(&mut *conn, course_id, course_update.deleted_mappers).await?;

            let course_mapper_count =
                database::count!(&mut *conn, "CourseMappers WHERE course_id = ?", course_id)
                    .await?;

            if course_mapper_count == 0 {
                return Err(UpdateMapError::MustHaveMappers);
            }

            if let Some(update) = course_update.filter_updates.vanilla {
                update_course_filter(&mut *conn, course_id, update, Mode::Vanilla).await?;
            }

            if let Some(update) = course_update.filter_updates.classic {
                update_course_filter(&mut *conn, course_id, update, Mode::Classic).await?;
            }
        }

        Ok(true)
    })
    .await
}

#[tracing::instrument(level = "debug", skip(conn), ret(level = "debug"), err(level = "debug"))]
async fn invalidate_old_map_rows(
    conn: &mut database::Connection,
    name: &str,
) -> database::Result<u64> {
    sqlx::query!("UPDATE Maps SET state = -1 WHERE name = ?", name)
        .execute(conn)
        .await
        .map(|result| result.rows_affected())
        .map_err(database::Error::from)
}

#[tracing::instrument(level = "debug", skip(conn), ret(level = "debug"), err(level = "debug"))]
async fn insert_map(
    conn: &mut database::Connection,
    workshop_id: WorkshopId,
    name: &str,
    description: Option<&str>,
    state: MapState,
    vpk_checksum: MapChecksum,
) -> database::Result<MapId> {
    sqlx::query!(
        "INSERT INTO Maps (workshop_id, name, description, state, vpk_checksum)
        VALUES (?, ?, ?, ?, ?)
        RETURNING id",
        workshop_id,
        name,
        description,
        state,
        vpk_checksum,
    )
    .fetch_one(conn)
    .await
    .and_then(|row| row.try_get(0))
    .map_err(database::Error::from)
}

#[tracing::instrument(level = "debug", skip(conn), err(level = "debug"))]
async fn insert_mappers(
    conn: &mut database::Connection,
    map_id: MapId,
    mappers: &[PlayerId],
) -> database::Result<()> {
    if mappers.is_empty() {
        return Ok(());
    }

    let mut query = QueryBuilder::new("INSERT INTO Mappers (map_id, player_id)");

    query.push_values(mappers, |mut query, player_id| {
        query.push_bind(map_id).push_bind(player_id);
    });

    query.build().execute(conn).await?;

    Ok(())
}

#[tracing::instrument(level = "debug", skip(conn), err(level = "debug"))]
async fn delete_mappers(
    conn: &mut database::Connection,
    map_id: MapId,
    mappers: &[PlayerId],
) -> database::Result<()> {
    if mappers.is_empty() {
        return Ok(());
    }

    let mut query = QueryBuilder::new("DELETE FROM Mappers");

    query.push(" WHERE map_id = ").push_bind(map_id);
    query.push(" AND player_id IN ");
    query.push_tuples(mappers, |mut query, player_id| {
        query.push_bind(player_id);
    });

    query.build().execute(conn).await?;

    Ok(())
}

#[tracing::instrument(level = "debug", skip(conn), err(level = "debug"))]
async fn insert_courses(
    conn: &mut database::Connection,
    map_id: MapId,
    courses: &[NewCourse],
) -> database::Result<()> {
    let mut query = QueryBuilder::new("INSERT INTO Courses (map_id, name, description)");

    query.push_values(courses, |mut query, course| {
        query.push_bind(map_id);
        query.push_bind(course.name.as_str());
        query.push_bind(course.description.as_deref());
    });

    query.push("RETURNING id");

    let course_ids = query
        .build_query_scalar::<CourseId>()
        .fetch_all(&mut *conn)
        .await?;

    assert_eq!(course_ids.len(), courses.len());

    let mappers = iter::zip(&course_ids, courses).flat_map(|(&course_id, course)| {
        iter::zip(iter::repeat(course_id), course.mappers.iter().copied())
    });

    insert_course_mappers(&mut *conn, mappers).await?;

    let mut query = QueryBuilder::new(
        "INSERT INTO CourseFilters (
           course_id,
           `mode`,
           nub_tier,
           pro_tier,
           state,
           notes
        )",
    );

    let filters = iter::zip(&course_ids, courses)
        .flat_map(|(&course_id, course)| iter::zip(iter::repeat(course_id), &course.filters));

    query.push_values(filters, |mut query, (course_id, (mode, filter))| {
        query.push_bind(course_id);
        query.push_bind(mode);
        query.push_bind(filter.nub_tier);
        query.push_bind(filter.pro_tier);
        query.push_bind(filter.state);
        query.push_bind(filter.notes.as_deref());
    });

    query.build().execute(&mut *conn).await?;

    Ok(())
}

#[tracing::instrument(level = "debug", skip(conn, mappers), err(level = "debug"))]
async fn insert_course_mappers(
    conn: &mut database::Connection,
    mappers: impl Iterator<Item = (CourseId, PlayerId)>,
) -> database::Result<()> {
    let mut query = QueryBuilder::new("INSERT INTO CourseMappers (course_id, player_id)");

    query.push_values(mappers, |mut query, (course_id, player_id)| {
        query.push_bind(course_id).push_bind(player_id);
    });

    query.build().execute(conn).await?;

    Ok(())
}

#[tracing::instrument(level = "debug", skip(conn), err(level = "debug"))]
async fn delete_course_mappers(
    conn: &mut database::Connection,
    course_id: CourseId,
    mappers: &[PlayerId],
) -> database::Result<()> {
    if mappers.is_empty() {
        return Ok(());
    }

    let mut query = QueryBuilder::new("DELETE FROM CourseMappers");

    query.push(" WHERE course_id = ").push_bind(course_id);
    query.push(" AND player_id IN ");
    query.push_tuples(mappers, |mut query, player_id| {
        query.push_bind(player_id);
    });

    query.build().execute(conn).await?;

    Ok(())
}

#[tracing::instrument(level = "debug", skip(conn), err(level = "debug"))]
async fn update_course_filter(
    conn: &mut database::Connection,
    course_id: CourseId,
    update: FilterUpdate<'_>,
    mode: Mode,
) -> database::Result<()> {
    sqlx::query!(
        "UPDATE CourseFilters
         SET nub_tier = COALESCE(?, nub_tier),
             pro_tier = COALESCE(?, pro_tier),
             state = COALESCE(?, state),
             notes = COALESCE(?, notes)
         WHERE course_id = ?
         AND mode = ?",
        update.nub_tier,
        update.pro_tier,
        update.state,
        update.notes,
        course_id,
        mode,
    )
    .execute(conn)
    .await?;

    Ok(())
}

impl<'a> IntoIterator for &'a NewCourseFilters {
    type Item = (Mode, &'a NewCourseFilter);
    type IntoIter = array::IntoIter<(Mode, &'a NewCourseFilter), 2>;

    fn into_iter(self) -> Self::IntoIter {
        [
            (Mode::Vanilla, &self.vanilla),
            (Mode::Classic, &self.classic),
        ]
        .into_iter()
    }
}

mod macros {
    macro_rules! select {
        ($conn:expr) => { self::macros::select!($conn, "") };
        ( $conn:expr, $($extra:tt)* ) => {{
            let raw_stream = sqlx::query!(
                "SELECT
                   m.id AS `id: MapId`,
                   m.workshop_id AS `workshop_id: WorkshopId`,
                   m.name,
                   m.description,
                   m.state AS `state: MapState`,
                   m.vpk_checksum AS `vpk_checksum: MapChecksum`,
                   mapper.id AS `mapper_id: PlayerId`,
                   mapper.name AS mapper_name,
                   c.id AS `course_id: CourseId`,
                   c.name AS course_name,
                   c.description AS course_description,
                   cmapper.id AS `course_mapper_id: PlayerId`,
                   cmapper.name AS course_mapper_name,
                   cf.id AS `filter_id: CourseFilterId`,
                   cf.mode AS `filter_mode: Mode`,
                   cf.nub_tier AS `nub_tier: Tier`,
                   cf.pro_tier AS `pro_tier: Tier`,
                   cf.state AS `filter_state: CourseFilterState`,
                   cf.notes AS filter_notes,
                   m.approved_at
                 FROM Maps AS m
                 JOIN Mappers ON Mappers.map_id = m.id
                 JOIN Players AS mapper ON mapper.id = Mappers.player_id
                 JOIN Courses AS c ON c.map_id = m.id
                 JOIN CourseMappers ON CourseMappers.course_id = c.id
                 JOIN Players AS cmapper ON cmapper.id = CourseMappers.player_id
                 JOIN CourseFilters AS cf ON cf.course_id = c.id "
                + $($extra)*
            )
            .fetch($conn)
            .map_err(database::Error::from)
            .map_ok(|row| RawMap {
                id: row.id,
                workshop_id: row.workshop_id,
                name: row.name,
                description: row.description,
                state: row.state,
                vpk_checksum: row.vpk_checksum,
                mappers: BTreeSet::from_iter([PlayerInfo { id: row.mapper_id, name: row.mapper_name }]),
                courses: BTreeMap::from_iter([(row.course_id, RawCourse {
                    id: row.course_id,
                    name: row.course_name,
                    description: row.course_description,
                    mappers: BTreeSet::from_iter([PlayerInfo {
                        id: row.course_mapper_id,
                        name: row.course_mapper_name,
                    }]),
                    filters: {
                        let filter = CourseFilter {
                            id: row.filter_id,
                            nub_tier: row.nub_tier,
                            pro_tier: row.pro_tier,
                            state: row.filter_state,
                            notes: row.filter_notes,
                        };

                        match row.filter_mode {
                            Mode::Vanilla => RawCourseFilters { vanilla: Some(filter), classic: None },
                            Mode::Classic => RawCourseFilters { vanilla: None, classic: Some(filter) },
                        }
                    },
                })]),
                approved_at: row.approved_at.into(),
            });

            GetMapsStream::new(raw_stream)
        }};
    }

    pub(super) use select;
}
