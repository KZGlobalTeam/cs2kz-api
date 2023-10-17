use utoipa::ToResponse;

#[derive(ToResponse)]
#[response(description = "Request body is malformed in some way.")]
pub struct BadRequest;

#[derive(ToResponse)]
#[response(description = "API token is missing / invalid.")]
pub struct Unauthorized;

#[derive(ToResponse)]
#[response(description = "There was an issue with the database.")]
pub struct Database;
