use {
	axum::{
		headers::{self, Header},
		http::{HeaderName, HeaderValue},
	},
	serde::Deserialize,
};

#[derive(Debug, Deserialize)]
pub struct ApiKey(pub u32);

static API_KEY_HEADER_NAME: HeaderName = HeaderName::from_static("api-key");

impl Header for ApiKey {
	fn name() -> &'static HeaderName {
		&API_KEY_HEADER_NAME
	}

	fn decode<'i, I>(values: &mut I) -> Result<Self, headers::Error>
	where
		Self: Sized,
		I: Iterator<Item = &'i HeaderValue>, {
		let value = values
			.next()
			.ok_or_else(headers::Error::invalid)?
			.to_str()
			.map_err(|_| headers::Error::invalid())?
			.parse::<u32>()
			.map_err(|_| headers::Error::invalid())?;

		Ok(Self(value))
	}

	fn encode<E: Extend<HeaderValue>>(&self, values: &mut E) {
		let s = self.0.to_string();
		let value = HeaderValue::from_str(&s).expect("u32 is valid ASCII");

		values.extend(std::iter::once(value));
	}
}
