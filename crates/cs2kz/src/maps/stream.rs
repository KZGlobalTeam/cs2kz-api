//! A custom [`Stream`] implementation for fetching [`Map`]s from the database.
//!
//! [`Map`] is a very nested data structure, which can't be expressed very well in SQL. Because of
//! this, the rows returned from our queries contain a lot of redundant data and must be merged in
//! application code. [`GetMapsStream`] does this transparently in its [`Stream`] implementation,
//! which will lazily fetch rows from the database and merge them together appropriately. The
//! logical correctness of this depends on the query defined in [`super::macros`].

use std::collections::BTreeSet;
use std::collections::btree_map::{self, BTreeMap};
use std::mem;
use std::pin::Pin;
use std::task::{Poll, ready};

use futures_util::Stream;

use crate::database;
use crate::maps::{
    Checksum,
    Course,
    CourseFilter,
    CourseFilters,
    CourseId,
    GetMapsError,
    Map,
    MapId,
    MapState,
};
use crate::players::PlayerInfo;
use crate::steam::WorkshopId;
use crate::time::Timestamp;

#[pin_project]
#[derive(Debug)]
pub(super) struct GetMapsStream<S> {
    #[pin]
    #[debug(skip)]
    stream: S,
    current: Option<RawMap>,
}

#[derive(Debug)]
pub(super) struct RawMap {
    pub(super) id: MapId,
    pub(super) workshop_id: WorkshopId,
    pub(super) name: String,
    pub(super) description: Option<String>,
    pub(super) state: MapState,
    pub(super) vpk_checksum: Checksum,
    pub(super) mappers: BTreeSet<PlayerInfo>,
    pub(super) courses: BTreeMap<CourseId, RawCourse>,
    pub(super) approved_at: Timestamp,
}

#[derive(Debug)]
pub(super) struct RawCourse {
    pub(super) id: CourseId,
    pub(super) name: String,
    pub(super) description: Option<String>,
    pub(super) mappers: BTreeSet<PlayerInfo>,
    pub(super) filters: RawCourseFilters,
}

#[derive(Debug)]
pub(super) struct RawCourseFilters {
    pub(super) vanilla: Option<CourseFilter>,
    pub(super) classic: Option<CourseFilter>,
}

impl<S: Stream<Item = database::Result<RawMap>>> GetMapsStream<S> {
    pub(super) fn new(stream: S) -> Self {
        Self { stream, current: None }
    }
}

impl<S: Stream<Item = database::Result<RawMap>>> Stream for GetMapsStream<S> {
    type Item = Result<Map, GetMapsError>;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let mut me = self.project();

        loop {
            let Some(current) = me.current else {
                match ready!(me.stream.as_mut().poll_next(cx)) {
                    // no current map AND stream is done -> we're done
                    None => break Poll::Ready(None),
                    // no current map, stream returned an error -> yield error
                    Some(Err(error)) => break Poll::Ready(Some(Err(error.into()))),
                    // no current map, stream yielded next map -> set current and try again
                    Some(Ok(next)) => {
                        *me.current = Some(next);
                        continue;
                    },
                }
            };

            let next = match ready!(me.stream.as_mut().poll_next(cx)) {
                // we have a current map, but stream is done -> yield current
                None => break Poll::Ready(me.current.take().map(|raw| Ok(raw.into()))),
                // we have a current map, but stream yielded an error -> yield error
                Some(Err(error)) => break Poll::Ready(Some(Err(error.into()))),
                // we have a current map, stream yielded next map
                Some(Ok(next)) => next,
            };

            // next map is the start of a different map
            // -> replace current with next and yield old current
            if next.id != current.id {
                break Poll::Ready(Some(Ok(mem::replace(current, next).into())));
            }

            // the IDs are equal -> merge them and try again
            merge(current, next);
        }
    }
}

impl From<RawMap> for Map {
    fn from(raw: RawMap) -> Self {
        Self {
            id: raw.id,
            workshop_id: raw.workshop_id,
            name: raw.name,
            description: raw.description,
            state: raw.state,
            vpk_checksum: raw.vpk_checksum,
            mappers: Vec::from_iter(raw.mappers),
            courses: raw.courses.into_values().map(Course::from).collect(),
            approved_at: raw.approved_at,
        }
    }
}

impl From<RawCourse> for Course {
    fn from(raw: RawCourse) -> Self {
        Course {
            id: raw.id,
            name: raw.name,
            description: raw.description,
            mappers: Vec::from_iter(raw.mappers),
            filters: CourseFilters {
                vanilla: raw
                    .filters
                    .vanilla
                    .expect("filter should exist at this point"),
                classic: raw
                    .filters
                    .classic
                    .expect("filter should exist at this point"),
            },
        }
    }
}

fn merge(current: &mut RawMap, next: RawMap) {
    current.mappers.extend(next.mappers);

    for (new_course_id, new_course) in next.courses {
        match current.courses.entry(new_course_id) {
            btree_map::Entry::Vacant(entry) => {
                entry.insert(new_course);
            },
            btree_map::Entry::Occupied(mut entry) => {
                merge_courses(entry.get_mut(), new_course);
            },
        }
    }
}

fn merge_courses(old: &mut RawCourse, new: RawCourse) {
    old.mappers.extend(new.mappers);

    match (&mut old.filters, new.filters) {
        (
            RawCourseFilters { vanilla: old_vanilla @ None, classic: None },
            RawCourseFilters { vanilla: new_vanilla @ Some(_), classic: None },
        ) => {
            *old_vanilla = new_vanilla;
        },
        (
            RawCourseFilters { vanilla: Some(_), classic: old_classic @ None },
            RawCourseFilters { vanilla: None, classic: new_classic @ Some(_) },
        ) => {
            *old_classic = new_classic;
        },
        _ => {},
    }
}
