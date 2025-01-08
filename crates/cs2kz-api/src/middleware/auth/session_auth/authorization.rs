use std::convert::Infallible;

use axum::RequestExt;
use axum::extract::Request;
use axum::response::{IntoResponse, Response};
use axum_extra::either::Either;
use cs2kz::Context;
use cs2kz::players::PlayerId;
use cs2kz::servers::{GetServersError, ServerId};
use cs2kz::users::Permissions;

use crate::extract::path::{Path, PathRejection};
use crate::middleware::auth::session_auth::Session;
use crate::response::ErrorResponse;

pub trait AuthorizeSession: Clone + Send + Sync + 'static {
    type Rejection: IntoResponse;

    async fn authorize_session(
        &mut self,
        request: &mut Request,
        session: &Session,
    ) -> Result<(), Self::Rejection>;

    fn or<U: AuthorizeSession>(self, alternative: U) -> Or<Self, U> {
        Or { left: self, right: alternative }
    }
}

#[derive(Debug, Clone)]
pub struct Noop;

impl AuthorizeSession for Noop {
    type Rejection = Infallible;

    async fn authorize_session(
        &mut self,
        _: &mut Request,
        _: &Session,
    ) -> Result<(), Self::Rejection> {
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Or<L, R> {
    left: L,
    right: R,
}

impl<L, R> AuthorizeSession for Or<L, R>
where
    L: AuthorizeSession,
    R: AuthorizeSession,
{
    type Rejection = Either<L::Rejection, R::Rejection>;

    async fn authorize_session(
        &mut self,
        request: &mut Request,
        session: &Session,
    ) -> Result<(), Self::Rejection> {
        self.left
            .authorize_session(request, session)
            .await
            .map_err(Either::E1)?;

        self.right
            .authorize_session(request, session)
            .await
            .map_err(Either::E2)?;

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct HasPermissions {
    required_permissions: Permissions,
}

#[derive(Debug, Display, Error)]
#[display("you do not have the required permissions to make this request")]
pub struct InsufficientPermissions {
    required: Permissions,
    actual: Permissions,
}

impl HasPermissions {
    pub fn new(required_permissions: impl Into<Permissions>) -> Self {
        Self {
            required_permissions: required_permissions.into(),
        }
    }
}

impl AuthorizeSession for HasPermissions {
    type Rejection = InsufficientPermissions;

    async fn authorize_session(
        &mut self,
        _: &mut Request,
        session: &Session,
    ) -> Result<(), Self::Rejection> {
        let required = self.required_permissions;
        let actual = session.user().permissions();

        if !actual.contains(required) {
            return Err(InsufficientPermissions { required, actual });
        }

        Ok(())
    }
}

impl IntoResponse for InsufficientPermissions {
    fn into_response(self) -> Response {
        ErrorResponse::unauthorized().into_response()
    }
}

#[derive(Debug, Clone)]
pub struct IsServerOwner {
    cx: Context,
}

#[derive(Debug, Display, Error, From)]
pub enum IsServerOwnerRejection {
    #[display("failed to extract server ID path parameter")]
    ExtractServerId(PathRejection<ServerId>),

    #[display("invalid server ID")]
    ServerNotFound,

    #[display("failed to get server info")]
    GetServer(GetServersError),

    #[display("requesting user is not the server owner")]
    NotServerOwner,
}

impl IsServerOwner {
    pub fn new(cx: Context) -> Self {
        Self { cx }
    }
}

impl AuthorizeSession for IsServerOwner {
    type Rejection = IsServerOwnerRejection;

    async fn authorize_session(
        &mut self,
        request: &mut Request,
        session: &Session,
    ) -> Result<(), Self::Rejection> {
        let Path(server_id) = request.extract_parts().await?;
        let server = cs2kz::servers::get_by_id(&self.cx, server_id)
            .await?
            .ok_or(IsServerOwnerRejection::ServerNotFound)?;

        if server.owner.id != session.user().id() {
            return Err(IsServerOwnerRejection::NotServerOwner);
        }

        Ok(())
    }
}

impl IntoResponse for IsServerOwnerRejection {
    fn into_response(self) -> Response {
        match self {
            Self::ExtractServerId(rejection) => return rejection.into_response(),
            Self::ServerNotFound => ErrorResponse::not_found(),
            Self::GetServer(error) => ErrorResponse::internal_server_error(error),
            Self::NotServerOwner => ErrorResponse::unauthorized(),
        }
        .into_response()
    }
}

#[derive(Debug, Clone)]
pub struct IsPlayer {
    _priv: (),
}

#[derive(Debug, Display, Error, From)]
pub enum IsPlayerRejection {
    #[display("failed to extract player ID")]
    ExtractPlayerId(PathRejection<PlayerId>),

    #[display("you can only update your own preferences")]
    IsNotPlayer,
}

impl IsPlayer {
    pub fn new() -> Self {
        Self { _priv: () }
    }
}

impl AuthorizeSession for IsPlayer {
    type Rejection = IsPlayerRejection;

    async fn authorize_session(
        &mut self,
        request: &mut Request,
        session: &Session,
    ) -> Result<(), Self::Rejection> {
        let Path(player_id) = request.extract_parts().await?;

        if session.user().id().as_ref() != player_id.as_ref() {
            return Err(IsPlayerRejection::IsNotPlayer);
        }

        Ok(())
    }
}

impl IntoResponse for IsPlayerRejection {
    fn into_response(self) -> Response {
        match self {
            Self::ExtractPlayerId(rejection) => rejection.into_response(),
            Self::IsNotPlayer => ErrorResponse::unauthorized().into_response(),
        }
    }
}
