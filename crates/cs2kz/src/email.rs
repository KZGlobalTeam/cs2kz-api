/// An email address.
#[derive(Debug, Display, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct Email(lettre::address::Address);

crate::database::impl_traits!(Email as str => {
    fn encode<'a>(self, out: &'a str) {
        out = self.0.as_ref();
    }

    fn decode(value: String) -> Result<Self, BoxError> {
        Ok(Self(value.try_into()?))
    }
});
