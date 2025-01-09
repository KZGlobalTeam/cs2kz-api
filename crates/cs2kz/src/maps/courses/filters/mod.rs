use std::fmt;
use std::num::NonZero;

use futures_util::{Stream, TryStreamExt};

use self::stream::GetCourseFiltersStream;
use crate::maps::{CourseFilter, CourseFilters, MapId};
use crate::mode::Mode;
use crate::{Context, database};

mod stream;

define_id_type! {
    /// A unique identifier for course filters.
    #[derive(sqlx::Type)]
    #[sqlx(transparent)]
    pub struct CourseFilterId(NonZero<u16>);
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, sqlx::Type, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Tier {
    VeryEasy = 1,
    Easy = 2,
    Medium = 3,
    Advanced = 4,
    Hard = 5,
    VeryHard = 6,
    Extreme = 7,
    Death = 8,
    Unfeasible = 9,
    Impossible = 10,
}

#[repr(i8)]
#[derive(Debug, Clone, Copy, sqlx::Type, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CourseFilterState {
    Unranked = -1,
    Pending = 0,
    Ranked = 1,
}

#[derive(Debug, Default)]
pub struct GetCourseFiltersParams {
    /// Only return filters that belong to approved maps.
    pub approved_only: bool,

    /// Only return filters that belong to this map.
    pub map_id: Option<MapId>,

    /// Only return filters with an ID `>= min_id`.
    pub min_id: Option<CourseFilterId>,
}

#[derive(Debug, Display, Error, From)]
#[display("failed to get course filters")]
#[from(forward)]
pub struct GetCourseFiltersError(database::Error);

#[tracing::instrument(skip(cx), err(level = "debug"))]
pub async fn get_by_id(
    cx: &Context,
    filter_id: CourseFilterId,
) -> Result<Option<(Mode, CourseFilter)>, GetCourseFiltersError> {
    sqlx::query!(
        "SELECT
           cf.id AS `id: CourseFilterId`,
           cf.mode AS `mode: Mode`,
           cf.nub_tier AS `nub_tier: Tier`,
           cf.pro_tier AS `pro_tier: Tier`,
           cf.state AS `state: CourseFilterState`,
           cf.notes
         FROM CourseFilters AS cf
         JOIN Courses AS c ON c.id = cf.course_id
         JOIN Maps AS m ON m.id = c.map_id
         WHERE cf.id = ?",
        filter_id,
    )
    .fetch_optional(cx.database().as_ref())
    .await
    .map_err(GetCourseFiltersError::from)
    .map(|row| {
        row.map(|row| {
            (row.mode, CourseFilter {
                id: row.id,
                nub_tier: row.nub_tier,
                pro_tier: row.pro_tier,
                state: row.state,
                notes: row.notes,
            })
        })
    })
}

#[tracing::instrument(skip(cx))]
pub fn get(
    cx: &Context,
    GetCourseFiltersParams { approved_only, map_id, min_id }: GetCourseFiltersParams,
) -> impl Stream<Item = Result<CourseFilters, GetCourseFiltersError>> {
    let raw_stream = sqlx::query_as!(
        CourseFilter,
        "SELECT
           cf.id AS `id: CourseFilterId`,
           cf.nub_tier AS `nub_tier: Tier`,
           cf.pro_tier AS `pro_tier: Tier`,
           cf.state AS `state: CourseFilterState`,
           cf.notes
         FROM CourseFilters AS cf
         JOIN Courses AS c ON c.id = cf.course_id
         JOIN Maps AS m ON m.id = c.map_id
         WHERE m.id = COALESCE(?, m.id)
         AND cf.id >= COALESCE(?, 1)
         AND (? OR m.state = 1)
         ORDER BY cf.id ASC, cf.mode ASC",
        map_id,
        min_id,
        !approved_only,
    )
    .fetch(cx.database().as_ref())
    .map_err(database::Error::from);

    GetCourseFiltersStream::new(raw_stream)
}

impl Tier {
    pub fn is_humanly_possible(&self) -> bool {
        *self <= Self::Death
    }
}

impl<'de> serde::Deserialize<'de> for Tier {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, Unexpected};

        struct TierVisitor;

        impl de::Visitor<'_> for TierVisitor {
            type Value = Tier;

            fn expecting(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(fmt, "a tier")
            }

            fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                match value {
                    1 => Ok(Tier::VeryEasy),
                    2 => Ok(Tier::Easy),
                    3 => Ok(Tier::Medium),
                    4 => Ok(Tier::Advanced),
                    5 => Ok(Tier::Hard),
                    6 => Ok(Tier::VeryHard),
                    7 => Ok(Tier::Extreme),
                    8 => Ok(Tier::Death),
                    9 => Ok(Tier::Unfeasible),
                    10 => Ok(Tier::Impossible),
                    _ => Err(E::invalid_value(Unexpected::Unsigned(value), &self)),
                }
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                if let Ok(int) = value.parse::<u64>() {
                    return self.visit_u64(int);
                }

                match value {
                    "very-easy" => Ok(Tier::VeryEasy),
                    "easy" => Ok(Tier::Easy),
                    "medium" => Ok(Tier::Medium),
                    "advanced" => Ok(Tier::Advanced),
                    "hard" => Ok(Tier::Hard),
                    "very-hard" => Ok(Tier::VeryHard),
                    "extreme" => Ok(Tier::Extreme),
                    "death" => Ok(Tier::Death),
                    "unfeasible" => Ok(Tier::Unfeasible),
                    "impossible" => Ok(Tier::Impossible),
                    _ => Err(E::invalid_value(Unexpected::Str(value), &self)),
                }
            }
        }

        deserializer.deserialize_any(TierVisitor)
    }
}

impl CourseFilterState {
    pub fn is_ranked(&self) -> bool {
        matches!(self, Self::Ranked)
    }
}

impl<'de> serde::Deserialize<'de> for CourseFilterState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, Unexpected};

        struct CourseFilterStateVisitor;

        impl de::Visitor<'_> for CourseFilterStateVisitor {
            type Value = CourseFilterState;

            fn expecting(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(fmt, "a course filter state")
            }

            fn visit_i8<E>(self, value: i8) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                match value {
                    -1 => Ok(CourseFilterState::Unranked),
                    0 => Ok(CourseFilterState::Pending),
                    1 => Ok(CourseFilterState::Ranked),
                    _ => Err(E::invalid_value(Unexpected::Signed(value.into()), &self)),
                }
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                if let Ok(int) = value.parse::<i8>() {
                    return self.visit_i8(int);
                }

                match value {
                    "unranked" => Ok(CourseFilterState::Unranked),
                    "pending" => Ok(CourseFilterState::Pending),
                    "ranked" => Ok(CourseFilterState::Ranked),
                    _ => Err(E::invalid_value(Unexpected::Str(value), &self)),
                }
            }
        }

        deserializer.deserialize_any(CourseFilterStateVisitor)
    }
}
