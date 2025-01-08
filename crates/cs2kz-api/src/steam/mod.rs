use bytes::Bytes;
use http_body_util::BodyExt;

pub mod users;
pub use users::fetch_user;

pub mod maps;
pub use maps::{download_map, fetch_map_name};

#[derive(Debug, Display, Error, From)]
pub enum ApiError {
    #[display("failed to make http request")]
    Http(reqwest::Error),

    #[display("failed to buffer response body")]
    #[from(ignore)]
    BufferResponseBody {
        #[error(source)]
        error: reqwest::Error,
        response: http::response::Parts,
    },

    #[display("failed to deserialize response from Steam")]
    #[from(ignore)]
    DeserializeResponse {
        #[error(source)]
        error: serde_json::Error,
        response: http::Response<Bytes>,
    },
}

/// Makes a request to Steam's Web API.
#[tracing::instrument(skip(request), err(level = "debug"))]
async fn request<T>(request: reqwest::RequestBuilder) -> Result<T, ApiError>
where
    T: for<'de> serde::Deserialize<'de>,
{
    #[derive(Debug, serde::Deserialize)]
    struct ApiResponse<T> {
        response: T,
    }

    let response = request.send().await?;

    if let Err(error) = response.error_for_status_ref() {
        return Err(ApiError::Http(error));
    }

    let (response, body) = http::Response::from(response).into_parts();
    let body = match body.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(error) => return Err(ApiError::BufferResponseBody { error, response }),
    };

    serde_json::from_slice(&body[..])
        .map(|ApiResponse { response }| response)
        .map_err(|err| ApiError::DeserializeResponse {
            error: err,
            response: http::Response::from_parts(response, body),
        })
}
