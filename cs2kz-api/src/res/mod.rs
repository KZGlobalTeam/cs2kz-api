use utoipa::ToResponse;

pub mod player;

#[derive(ToResponse)]
#[response(description = "Request body is malformed in some way.")]
pub struct BadRequest;
