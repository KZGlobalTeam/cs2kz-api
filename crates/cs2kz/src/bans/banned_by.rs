use crate::servers::ServerId;
use crate::users::UserId;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case", tag = "type", content = "id")]
pub enum BannedBy {
    Server(ServerId),
    Admin(UserId),
}

crate::database::impl_traits!(BannedBy as u64 => {
    fn encode(self, out: u64) {
        out = match self {
            Self::Server(server_id) => server_id.into_inner().get().into(),
            Self::Admin(user_id) => user_id.as_ref().as_u64(),
        };
    }

    fn decode(value: u64) -> Result<Self, BoxError> {
        if let Ok(server_id) = ServerId::try_from(value) {
            Ok(Self::Server(server_id))
        } else {
            UserId::try_from(value)
                .map(Self::Admin)
                .map_err(crate::database::Error::decode)
                .map_err(Into::into)
        }
    }
});
