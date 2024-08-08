use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error
{
	#[error(transparent)]
	Database(#[from] sqlx::Error),
}
