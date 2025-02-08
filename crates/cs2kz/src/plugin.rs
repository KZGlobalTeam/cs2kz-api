//! [`cs2kz-metamod`] release metadata.
//!
//! The API keeps track of every version of the plugin, mostly to prevent servers running outdated
//! versions from submitting records/jumpstats.
//!
//! [`cs2kz-metamod`]: https://github.com/KZGlobalTeam/cs2kz-metamod

use std::num::NonZero;

use futures_util::TryStreamExt;
use sqlx::Row;

use crate::checksum::Checksum;
use crate::context::Context;
use crate::database::{self, QueryBuilder};
use crate::git::GitRevision;
use crate::mode::Mode;
use crate::pagination::{Limit, Offset, Paginated};
use crate::styles::Style;
use crate::time::Timestamp;

define_id_type! {
    /// A unique identifier for [`cs2kz-metamod`] releases.
    ///
    /// [`cs2kz-metamod`]: https://github.com/KZGlobalTeam/cs2kz-metamod
    #[cfg_attr(feature = "fake", derive(fake::Dummy))]
    #[derive(sqlx::Type)]
    #[sqlx(transparent)]
    pub struct PluginVersionId(NonZero<u16>);
}

/// [`cs2kz-metamod`] release metadata.
///
/// [`cs2kz-metamod`]: https://github.com/KZGlobalTeam/cs2kz-metamod
#[derive(Debug, serde::Serialize)]
pub struct PluginVersion {
    pub id: PluginVersionId,
    pub version: semver::Version,
    pub git_revision: GitRevision,
    pub published_at: Timestamp,
}

#[derive(Debug)]
pub struct NewPluginVersion<'a, M, S>
where
    M: Iterator<Item = NewMode<'a>>,
    S: Iterator<Item = NewStyle<'a>>,
{
    pub version: &'a semver::Version,
    pub git_revision: &'a GitRevision,
    pub linux_checksum: &'a Checksum,
    pub windows_checksum: &'a Checksum,
    pub is_cutoff: bool,
    pub modes: M,
    pub styles: S,
}

#[derive(Debug)]
pub struct NewMode<'a> {
    pub mode: Mode,
    pub linux_checksum: &'a Checksum,
    pub windows_checksum: &'a Checksum,
}

#[derive(Debug)]
pub struct NewStyle<'a> {
    pub style: Style,
    pub linux_checksum: &'a Checksum,
    pub windows_checksum: &'a Checksum,
}

#[derive(Debug)]
pub struct GetPluginVersionsParams<'a> {
    /// Only include versions which meet this requirement.
    pub version_req: Option<&'a semver::VersionReq>,
    pub limit: Limit<250, 10>,
    pub offset: Offset,
}

#[derive(Debug, Display, Error, From)]
pub enum PublishPluginVersionError {
    #[display("version has already been published")]
    VersionAlreadyPublished,

    #[display("version is older than the latest version ({latest})")]
    #[error(ignore)]
    #[from(ignore)]
    VersionOlderThanLatest { latest: semver::Version },

    #[display("{_0}")]
    Database(database::Error),
}

#[derive(Debug, Display, Error, From)]
#[display("failed to get plugin versions")]
#[from(forward)]
pub struct GetPluginVersionsError(database::Error);

#[derive(Debug, Display, Error, From)]
#[display("failed to delete plugin version")]
#[from(forward)]
pub struct DeletePluginVersionError(database::Error);

#[tracing::instrument(skip(cx, modes, styles), ret(level = "debug"), err(level = "debug"))]
pub async fn publish_version<'a, M, S>(
    cx: &Context,
    NewPluginVersion {
        version,
        git_revision,
        linux_checksum,
        windows_checksum,
        is_cutoff,
        modes,
        styles,
    }: NewPluginVersion<'a, M, S>,
) -> Result<PluginVersionId, PublishPluginVersionError>
where
    M: Iterator<Item = NewMode<'a>>,
    S: Iterator<Item = NewStyle<'a>>,
{
    if let Some(latest) = get_latest_version(cx)
        .await
        .map_err(|GetPluginVersionsError(error)| PublishPluginVersionError::Database(error))?
        .filter(|latest| latest.version > *version)
    {
        return Err(PublishPluginVersionError::VersionOlderThanLatest { latest: latest.version });
    }

    cx.database_transaction(async move |conn| {
        let plugin_version_id = sqlx::query!(
            "INSERT INTO PluginVersions (
               major,
               minor,
               patch,
               pre,
               build,
               git_revision,
               linux_checksum,
               windows_checksum,
               is_cutoff
             )
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
             RETURNING id",
            version.major,
            version.minor,
            version.patch,
            version.pre.as_str(),
            version.build.as_str(),
            git_revision,
            linux_checksum,
            windows_checksum,
            is_cutoff,
        )
        .fetch_one(&mut *conn)
        .await
        .and_then(|row| row.try_get(0))
        .map_err(database::Error::from)
        .map_err(|err| {
            if err.is_unique_violation_of("UC_semver") || err.is_unique_violation_of("git_revision")
            {
                PublishPluginVersionError::VersionAlreadyPublished
            } else {
                PublishPluginVersionError::Database(err)
            }
        })?;

        let mut query = QueryBuilder::new("INSERT INTO ModeChecksums ");

        query.push_values(modes, |mut query, mode| {
            query.push_bind(mode.mode);
            query.push_bind(plugin_version_id);
            query.push_bind(mode.linux_checksum);
            query.push_bind(mode.windows_checksum);
        });

        query
            .build()
            .execute(&mut *conn)
            .await
            .map_err(database::Error::from)
            .map_err(PublishPluginVersionError::Database)?;

        let mut query = QueryBuilder::new("INSERT INTO StyleChecksums ");

        query.push_values(styles, |mut query, style| {
            query.push_bind(style.style);
            query.push_bind(plugin_version_id);
            query.push_bind(style.linux_checksum);
            query.push_bind(style.windows_checksum);
        });

        query
            .build()
            .execute(&mut *conn)
            .await
            .map_err(database::Error::from)
            .map_err(PublishPluginVersionError::Database)?;

        Ok(plugin_version_id)
    })
    .await
}

#[tracing::instrument(skip(cx), err(level = "debug"))]
pub async fn get_versions(
    cx: &Context,
    GetPluginVersionsParams { version_req, limit, offset }: GetPluginVersionsParams<'_>,
) -> Result<Paginated<Vec<PluginVersion>>, GetPluginVersionsError> {
    let mut total = 0;
    let mut plugin_versions = Vec::new();

    // MariaDB doesn't have any built-in functions to filter on SemVer versions, so we have to do
    // it ourselves.

    let mut stream = self::macros::select!(
        "ORDER BY published_at DESC
         LIMIT ?
         OFFSET ?",
        u64::MAX,
        offset.value(),
    )
    .fetch(cx.database().as_ref());

    while let Some(row) = stream.try_next().await.map_err(database::Error::from)? {
        let plugin_version = self::macros::parse_row!(row);

        if version_req.is_some_and(|req| !req.matches(&plugin_version.version)) {
            continue;
        }

        total += 1;

        if (plugin_versions.len() as u64) < limit.value() {
            plugin_versions.push(plugin_version);
        }
    }

    Ok(Paginated::new(total, plugin_versions))
}

#[tracing::instrument(skip(cx), fields(%version), err(level = "debug"))]
pub async fn get_version(
    cx: &Context,
    version: &semver::Version,
) -> Result<Option<PluginVersion>, GetPluginVersionsError> {
    self::macros::select!(
        "WHERE major = ?
         AND minor = ?
         AND patch = ?
         AND pre = ?
         AND build = ?",
        version.major,
        version.minor,
        version.patch,
        version.pre.as_str(),
        version.build.as_str(),
    )
    .fetch_optional(cx.database().as_ref())
    .await
    .map_err(GetPluginVersionsError::from)
    .and_then(|row| match row {
        None => Ok(None),
        Some(row) => Ok(Some(self::macros::parse_row!(row))),
    })
}

#[tracing::instrument(skip(cx), err(level = "debug"))]
pub async fn get_version_by_git_revision(
    cx: &Context,
    git_revision: &GitRevision,
) -> Result<Option<PluginVersion>, GetPluginVersionsError> {
    self::macros::select!("WHERE git_revision = ?", git_revision)
        .fetch_optional(cx.database().as_ref())
        .await
        .map_err(GetPluginVersionsError::from)
        .and_then(|row| match row {
            None => Ok(None),
            Some(row) => Ok(Some(self::macros::parse_row!(row))),
        })
}

#[tracing::instrument(skip(cx), err(level = "debug"))]
pub async fn get_latest_version(
    cx: &Context,
) -> Result<Option<PluginVersion>, GetPluginVersionsError> {
    self::macros::select!("ORDER BY published_at DESC LIMIT 1")
        .fetch_optional(cx.database().as_ref())
        .await
        .map_err(GetPluginVersionsError::from)
        .and_then(|row| match row {
            None => Ok(None),
            Some(row) => Ok(Some(self::macros::parse_row!(row))),
        })
}

#[tracing::instrument(skip(cx), err(level = "debug"))]
pub async fn is_valid_version(
    cx: &Context,
    id: PluginVersionId,
    checksum: &Checksum,
) -> database::Result<bool> {
    sqlx::query_scalar!(
        "SELECT COUNT(id) > 0 AS `is_valid: bool`
         FROM PluginVersions
         WHERE id = ?
         AND (linux_checksum = ? OR windows_checksum = ?)
         AND id >= COALESCE(
           (SELECT id FROM PluginVersions WHERE is_cutoff ORDER BY published_at DESC LIMIT 1),
           0
         )",
        id,
        checksum,
        checksum,
    )
    .fetch_one(cx.database().as_ref())
    .await
    .map_err(database::Error::from)
}

#[tracing::instrument(skip(cx), ret(level = "debug"), err(level = "debug"))]
pub async fn delete_version(
    cx: &Context,
    version: &semver::Version,
) -> Result<bool, DeletePluginVersionError> {
    sqlx::query!(
        "DELETE FROM PluginVersions
         WHERE major = ?
         AND minor = ?
         AND patch = ?
         AND pre = ?
         AND build = ?",
        version.major,
        version.minor,
        version.patch,
        version.pre.as_str(),
        version.build.as_str(),
    )
    .execute(cx.database().as_ref())
    .await
    .map(|result| result.rows_affected() > 0)
    .map_err(DeletePluginVersionError::from)
}

mod macros {
    macro_rules! select {
        ( $($extra:tt)* ) => {
            sqlx::query!(
                "SELECT
                   id AS `id: PluginVersionId`,
                   major AS `major: u64`,
                   minor AS `minor: u64`,
                   patch AS `patch: u64`,
                   pre,
                   build,
                   git_revision AS `git_revision: GitRevision`,
                   published_at
                 FROM PluginVersions "
                 + $($extra)*
            )
        };
    }

    macro_rules! parse_row {
        ($row:expr) => {
            PluginVersion {
                id: $row.id,
                version: semver::Version {
                    major: $row.major.into(),
                    minor: $row.minor.into(),
                    patch: $row.patch.into(),
                    pre: $row
                        .pre
                        .parse()
                        .map_err(|err| $crate::database::Error::decode_column("pre", err))?,
                    build: $row
                        .build
                        .parse()
                        .map_err(|err| $crate::database::Error::decode_column("build", err))?,
                },
                git_revision: $row.git_revision,
                published_at: $row.published_at.into(),
            }
        };
    }

    pub(super) use {parse_row, select};
}
