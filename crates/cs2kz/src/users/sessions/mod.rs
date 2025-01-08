use crate::time::Timestamp;
use crate::users::{Permissions, UserId};
use crate::{Context, database};

mod session_id;
pub use session_id::{ParseSessionIdError, SessionId};

#[derive(Debug)]
pub struct Session {
    pub id: SessionId,
    pub user_id: UserId,
    pub user_permissions: Permissions,
    pub expires_at: Timestamp,
}

#[derive(Debug)]
pub struct NewSession<'a> {
    pub user_id: UserId,
    pub user_name: &'a str,
    pub expires_at: Timestamp,
}

#[derive(Debug, Display, Error, From)]
pub enum GetSessionsError {
    #[display("session has expired")]
    #[error(ignore)]
    #[from(ignore)]
    Expired(Session),

    #[display("{_0}")]
    Database(database::Error),
}

#[derive(Debug, Display, Error, From)]
#[display("failed to update session")]
#[from(forward)]
pub struct UpdateSessionError(database::Error);

#[derive(Debug, Display, Error, From)]
#[display("failed to login user")]
#[from(forward)]
pub struct LoginError(database::Error);

#[tracing::instrument(skip(cx), err(level = "debug"))]
pub async fn get(cx: &Context, session_id: SessionId) -> Result<Option<Session>, GetSessionsError> {
    match sqlx::query_as!(
        Session,
        "SELECT
           s.id AS `id: SessionId`,
           u.id AS `user_id: UserId`,
           u.permissions AS `user_permissions: Permissions`,
           s.expires_at
         FROM UserSessions AS s
         JOIN Users AS u ON u.id = s.user_id
         WHERE s.id = ?",
        session_id,
    )
    .fetch_optional(cx.database().as_ref())
    .await
    {
        Ok(Some(session)) if session.expires_at > Timestamp::now() => Ok(Some(session)),
        Ok(Some(session)) => Err(GetSessionsError::Expired(session)),
        Ok(None) => Ok(None),
        Err(error) => Err(GetSessionsError::Database(error.into())),
    }
}

#[tracing::instrument(skip(cx), err(level = "debug"))]
pub async fn expire(cx: &Context, session_id: SessionId) -> Result<(), UpdateSessionError> {
    sqlx::query!(
        "UPDATE UserSessions
         SET expires_at = NOW()
         WHERE id = ?
         AND expires_at > NOW()",
        session_id,
    )
    .execute(cx.database().as_ref())
    .await?;

    Ok(())
}

#[tracing::instrument(skip(cx), err(level = "debug"))]
pub async fn expire_all(cx: &Context, user_id: UserId) -> Result<(), UpdateSessionError> {
    sqlx::query!(
        "UPDATE UserSessions SET expires_at = NOW()
         WHERE user_id = ?
         AND expires_at > NOW()",
        user_id,
    )
    .execute(cx.database().as_ref())
    .await?;

    Ok(())
}

#[tracing::instrument(skip(cx), err(level = "debug"))]
pub async fn extend(
    cx: &Context,
    session_id: SessionId,
    expires_after: time::Duration,
) -> Result<(), UpdateSessionError> {
    sqlx::query!(
        "UPDATE UserSessions
         SET expires_at = ?
         WHERE id = ?",
        Timestamp::now() + expires_after,
        session_id
    )
    .execute(cx.database().as_ref())
    .await?;

    Ok(())
}

#[tracing::instrument(skip(cx), ret(level = "debug"), err(level = "debug"))]
pub async fn login(
    cx: &Context,
    NewSession { user_id, user_name, expires_at }: NewSession<'_>,
) -> Result<SessionId, LoginError> {
    cx.database_transaction(async move |conn| {
        sqlx::query!(
            "INSERT INTO Users (id, name)
             VALUES (?, ?)
             ON DUPLICATE KEY
             UPDATE name = VALUES(name),
                    last_login_at = NOW()",
            user_id,
            user_name,
        )
        .execute(&mut *conn)
        .await?;

        let session_id = SessionId::new();

        sqlx::query!(
            "INSERT INTO UserSessions (id, user_id, expires_at)
             VALUES (?, ?, ?)",
            session_id,
            user_id,
            expires_at,
        )
        .execute(&mut *conn)
        .await?;

        Ok(session_id)
    })
    .await
}
