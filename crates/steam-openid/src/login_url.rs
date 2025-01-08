use serde::Serialize;
use serde::ser::{SerializeMap, Serializer};
use url::Url;

use crate::LOGIN_URL;

/// Constructs a URL for OpenID 2.0 login with Steam.
///
/// Steam will redirect the user to `return_to` after the login process is complete. `userdata`
/// will be injected into this URL such that Steam's request will include a `userdata` field in its
/// query parameters.
#[tracing::instrument(
    level = "trace",
    skip(userdata),
    fields(return_to = return_to.as_str()),
    ret(Display, level = "debug"),
    err(level = "debug"),
)]
pub fn login_url<T>(mut return_to: Url, userdata: &T) -> Result<Url, serde_urlencoded::ser::Error>
where
    T: ?Sized + Serialize,
{
    {
        #[derive(Serialize)]
        struct UserData<'a, T: ?Sized> {
            userdata: &'a T,
        }

        let mut query = return_to.query_pairs_mut();
        let serializer = serde_urlencoded::Serializer::new(&mut query);

        (UserData { userdata }).serialize(serializer)?;
    }

    let query_string = serde_urlencoded::to_string(&Form { return_to: &return_to })?;

    Ok(format!("{LOGIN_URL}?{query_string}")
        .parse()
        .expect("hard-coded URL with valid query string should be a valid URL"))
}

struct Form<'a> {
    return_to: &'a Url,
}

impl Serialize for Form<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut serializer = serializer.serialize_map(Some(6))?;
        let realm = {
            let return_to = self.return_to.as_str();
            let path_range = return_to
                .substr_range(self.return_to.path())
                .expect("`path` is derived from `return_to`");

            &return_to[..(if path_range.start == 0 {
                return_to.len()
            } else {
                path_range.start
            })]
        };

        serializer.serialize_entry("openid.ns", "http://specs.openid.net/auth/2.0")?;
        serializer.serialize_entry("openid.mode", "checkid_setup")?;

        for key in ["openid.identity", "openid.claimed_id"] {
            serializer
                .serialize_entry(key, "http://specs.openid.net/auth/2.0/identifier_select")?;
        }

        serializer.serialize_entry("openid.realm", realm)?;
        serializer.serialize_entry("openid.return_to", self.return_to)?;

        serializer.end()
    }
}
