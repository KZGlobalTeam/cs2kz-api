use crate::context::Context;
use crate::database;
use crate::time::Timestamp;

mod access_key;
pub use access_key::{AccessKey, ParseAccessKeyError};

#[derive(Debug)]
pub struct AccessKeyInfo {
    pub name: String,
    pub value: AccessKey,
    pub expires_at: Timestamp,
}

#[derive(Debug, Display, Error, From)]
pub enum GetAccessKeyInfoError {
    #[display("the key has expired")]
    #[error(ignore)]
    #[from(ignore)]
    Expired(AccessKeyInfo),

    #[display("{_0}")]
    Database(database::Error),
}

#[tracing::instrument(skip(cx), ret(level = "debug"), err(level = "debug"))]
pub async fn get(
    cx: &Context,
    key: &AccessKey,
) -> Result<Option<AccessKeyInfo>, GetAccessKeyInfoError> {
    match sqlx::query_as!(
        AccessKeyInfo,
        "SELECT
           name,
           value AS `value: AccessKey`,
           expires_at
         FROM AccessKeys
         WHERE value = ?",
        key,
    )
    .fetch_optional(cx.database().as_ref())
    .await
    {
        Err(error) => Err(GetAccessKeyInfoError::Database(error.into())),
        Ok(Some(info)) if info.expires_at <= Timestamp::now() => {
            Err(GetAccessKeyInfoError::Expired(info))
        },
        Ok(info) => Ok(info),
    }
}
