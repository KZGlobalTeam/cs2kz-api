use std::str::FromStr;

use ulid::Ulid;

#[derive(Debug, Clone, Copy, AsRef, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct AccessKey(Ulid);

#[derive(Debug, Display, Error, From)]
#[display("failed to parse access key: {_0}")]
pub struct ParseAccessKeyError(ulid::DecodeError);

impl AccessKey {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self(Ulid::new())
    }
}

impl FromStr for AccessKey {
    type Err = ParseAccessKeyError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        value.parse::<Ulid>().map(Self).map_err(ParseAccessKeyError)
    }
}

crate::database::impl_traits!(AccessKey as [u8] => {
    fn encode<'a>(self, out: &'a [u8]) {
        let bytes = self.0.to_bytes();
        out = &bytes[..];
    }

    fn decode<'a>(bytes: &'a [u8]) -> Result<Self, BoxError> {
        <[u8; 16]>::try_from(bytes)
            .map(Ulid::from_bytes)
            .map(Self)
            .map_err(Into::into)
    }
});
