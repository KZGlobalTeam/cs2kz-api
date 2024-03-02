use std::fmt::{self, Debug};

use jsonwebtoken::errors::Result;
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation};
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::auth::Jwt;

/// Any JWT related state.
pub struct State {
	pub header: Header,
	pub encoding_key: EncodingKey,
	pub decoding_key: DecodingKey,
	pub validation: Validation,
}

impl State {
	pub fn new(secret: &str) -> Result<Self> {
		let header = Header::default();
		let encoding_key = EncodingKey::from_base64_secret(secret)?;
		let decoding_key = DecodingKey::from_base64_secret(secret)?;
		let validation = Validation::default();

		Ok(Self { header, encoding_key, decoding_key, validation })
	}

	/// Encodes the given `payload` as a JWT using the API's secret.
	pub fn encode<T>(&self, payload: &Jwt<T>) -> Result<String>
	where
		T: Serialize,
	{
		jsonwebtoken::encode(&self.header, payload, &self.encoding_key)
	}

	/// Decodes the given `jwt` into the desired payload type `T`.
	pub fn decode<T>(&self, jwt: &str) -> Result<T>
	where
		T: DeserializeOwned,
	{
		jsonwebtoken::decode(jwt, &self.decoding_key, &self.validation).map(|token| token.claims)
	}
}

impl Debug for State {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("State").finish_non_exhaustive()
	}
}
