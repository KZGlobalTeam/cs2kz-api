use jsonwebtoken::errors::Result;
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation};
use smart_debug::SmartDebug;

#[derive(SmartDebug)]
pub struct State {
	#[debug(skip)]
	pub header: Header,

	#[debug(skip)]
	pub encoding_key: EncodingKey,

	#[debug(skip)]
	pub decoding_key: DecodingKey,

	#[debug(skip)]
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
