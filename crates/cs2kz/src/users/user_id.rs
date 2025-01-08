use steam_id::SteamId;

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, AsRef, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct UserId(#[serde(serialize_with = "SteamId::serialize_u64_stringified")] SteamId);

impl UserId {
    pub const fn new(steam_id: SteamId) -> Self {
        Self(steam_id)
    }
}

impl TryFrom<u64> for UserId {
    type Error = <SteamId as TryFrom<u64>>::Error;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        SteamId::try_from(value).map(Self)
    }
}

crate::database::impl_traits!(UserId as u64 => {
    fn encode(self, out: u64) {
        out = self.0.as_u64();
    }

    fn decode(value: u64) -> Result<Self, BoxError> {
        SteamId::from_u64(value)
            .map(Self)
            .map_err(Into::into)
    }
});
