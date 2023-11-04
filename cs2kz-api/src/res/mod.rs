use utoipa::ToResponse;

pub mod player;
pub mod bans;
pub mod maps;

#[derive(ToResponse)]
#[response(description = "Request body is malformed in some way.")]
pub struct BadRequest;
