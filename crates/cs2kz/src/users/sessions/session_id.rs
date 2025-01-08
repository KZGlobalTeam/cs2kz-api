use std::str::FromStr;

use ulid::Ulid;

#[derive(Debug, Display, Clone, Copy, AsRef, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct SessionId(Ulid);

impl SessionId {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self(Ulid::new())
    }
}

#[derive(Debug, Display, Error, From)]
#[display("failed to parse session ID: {_0}")]
pub struct ParseSessionIdError(ulid::DecodeError);

impl FromStr for SessionId {
    type Err = ParseSessionIdError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        value.parse::<Ulid>().map(Self).map_err(ParseSessionIdError)
    }
}

crate::database::impl_traits!(SessionId as [u8] => {
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
