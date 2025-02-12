use std::{future, str};

use bytes::Bytes;
use http_body::Body as HttpBody;
use http_body_util::BodyExt;
use serde::{Deserialize, Serialize};
use steam_id::SteamId;
use tower_service::Service;
use url::Url;

use crate::LOGIN_URL;

/// Payload sent by Steam after the login process is complete.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallbackPayload {
    #[serde(rename = "openid.ns")]
    pub ns: String,

    #[serde(rename = "openid.identity")]
    pub identity: Option<String>,

    #[debug("{:?}", claimed_id.as_str())]
    #[serde(rename = "openid.claimed_id")]
    pub claimed_id: Url,

    #[serde(rename = "openid.mode")]
    pub mode: String,

    #[debug("{:?}", return_to.as_str())]
    #[serde(rename = "openid.return_to")]
    pub return_to: Url,

    #[serde(rename = "openid.op_endpoint")]
    pub op_endpoint: String,

    #[serde(rename = "openid.response_nonce")]
    pub response_nonce: String,

    #[serde(rename = "openid.invalidate_handle")]
    pub invalidate_handle: Option<String>,

    #[serde(rename = "openid.assoc_handle")]
    pub assoc_handle: String,

    #[serde(rename = "openid.signed")]
    pub signed: String,

    #[serde(rename = "openid.sig")]
    pub sig: String,

    /// The serialized `userdata` injected by [`login_url()`].
    #[serde(skip_serializing)]
    pub userdata: String,
}

#[derive(Debug, Display, Error)]
pub enum VerifyCallbackPayloadError<HttpError, ResponseBody>
where
    ResponseBody: HttpBody<Error: std::error::Error + 'static>,
{
    #[display("`return_to` host does not match our host")]
    HostMismatch,

    #[display("HTTP client error")]
    HttpClient(HttpError),

    #[display("failed to make HTTP request to Steam")]
    HttpRequest(HttpError),

    #[display("HTTP request returned a bad status code ({})", response.status())]
    #[error(ignore)]
    BadStatus { response: http::Response<Bytes> },

    #[display("failed to buffer response body")]
    BufferResponseBody {
        #[error(source)]
        error: ResponseBody::Error,
        response: http::response::Parts,
    },

    #[display("invalid payload")]
    #[error(ignore)]
    InvalidPayload { response: http::Response<Bytes> },
}

impl CallbackPayload {
    #[tracing::instrument(skip(self, http_client), ret(level = "debug"), err(level = "debug"))]
    pub async fn verify<S, ResponseBody>(
        &mut self,
        expected_host: url::Host<&str>,
        mut http_client: S,
    ) -> Result<SteamId, VerifyCallbackPayloadError<S::Error, ResponseBody>>
    where
        S: Service<http::Request<Bytes>, Response = http::Response<ResponseBody>>,
        ResponseBody: HttpBody<Error: std::error::Error + 'static>,
    {
        if self.return_to.host() != Some(expected_host) {
            return Err(VerifyCallbackPayloadError::HostMismatch);
        }

        if self.mode != "check_authentication" {
            self.mode.clear();
            self.mode.push_str("check_authentication");
        }

        let payload = serde_urlencoded::to_string(&*self)
            .expect("`CallbackPayload` should always serialize properly");

        let request = http::Request::post(LOGIN_URL)
            .header(http::header::CONTENT_TYPE, mime::APPLICATION_WWW_FORM_URLENCODED.as_ref())
            .body(Bytes::from(payload))
            .expect("valid http request");

        future::poll_fn(|cx| http_client.poll_ready(cx))
            .await
            .map_err(VerifyCallbackPayloadError::HttpClient)?;

        let (response, body) = http_client
            .call(request)
            .await
            .map_err(VerifyCallbackPayloadError::HttpRequest)?
            .into_parts();

        let body = match body.collect().await {
            Ok(collected) => collected.to_bytes(),
            Err(error) => {
                return Err(VerifyCallbackPayloadError::BufferResponseBody { error, response });
            },
        };

        if !response.status.is_success() {
            if let Ok(body) = str::from_utf8(&body[..]) {
                tracing::debug!(
                    body,
                    status = response.status.as_u16(),
                    "Steam returned bad status",
                );
            }

            return Err(VerifyCallbackPayloadError::BadStatus {
                response: http::Response::from_parts(response, body),
            });
        }

        if !body[..]
            .rsplit(|&byte| byte == b'\n')
            .any(|line| line == b"is_valid:true")
        {
            return Err(VerifyCallbackPayloadError::InvalidPayload {
                response: http::Response::from_parts(response, body),
            });
        }

        Ok(self
            .claimed_id
            .path_segments()
            .and_then(Iterator::last)
            .and_then(|segment| segment.parse::<SteamId>().ok())
            .expect("invalid payload"))
    }
}
