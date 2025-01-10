use steam_id::SteamId;

#[derive(
    Debug,
    Display,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    AsRef,
    Into,
    serde::Serialize,
    serde::Deserialize
)]
#[cfg_attr(feature = "fake", derive(fake::Dummy))]
pub struct PlayerId(SteamId);

impl PlayerId {
    pub const fn new(steam_id: SteamId) -> Self {
        Self(steam_id)
    }
}

crate::database::impl_traits!(PlayerId as u64 => {
    fn encode(self, out: u64) {
        out = self.0.as_u64();
    }

    fn decode(value: u64) -> Result<Self, BoxError> {
        SteamId::from_u64(value)
            .map(Self)
            .map_err(Into::into)
    }
});
