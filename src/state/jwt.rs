use std::fmt;

use jsonwebtoken::errors::Result;
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation};

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
}

impl fmt::Debug for State {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("JWT State").finish()
	}
}
