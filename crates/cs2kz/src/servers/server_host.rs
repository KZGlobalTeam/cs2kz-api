use std::net::{Ipv4Addr, Ipv6Addr};

#[derive(Debug, Display, serde::Serialize)]
#[serde(untagged)]
pub enum ServerHost {
    Ipv4(Ipv4Addr),
    Ipv6(Ipv6Addr),
    Domain(String),
}

impl<DB> sqlx::Type<DB> for ServerHost
where
    DB: sqlx::Database,
    str: sqlx::Type<DB>,
{
    fn type_info() -> <DB as sqlx::Database>::TypeInfo {
        <str as sqlx::Type<DB>>::type_info()
    }

    fn compatible(ty: &<DB as sqlx::Database>::TypeInfo) -> bool {
        <str as sqlx::Type<DB>>::compatible(ty)
    }
}

impl<'q, DB: sqlx::Database> sqlx::Encode<'q, DB> for ServerHost
where
    for<'a> &'a str: sqlx::Encode<'q, DB>,
    String: sqlx::Encode<'q, DB>,
    Ipv4Addr: sqlx::Encode<'q, DB>,
    Ipv6Addr: sqlx::Encode<'q, DB>,
{
    fn encode_by_ref(
        &self,
        buf: &mut <DB as sqlx::Database>::ArgumentBuffer<'q>,
    ) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + Sync + Send>> {
        match self {
            Self::Ipv4(addr) => <Ipv4Addr as sqlx::Encode<'q, DB>>::encode_by_ref(addr, buf),
            Self::Ipv6(addr) => <Ipv6Addr as sqlx::Encode<'q, DB>>::encode_by_ref(addr, buf),
            Self::Domain(domain) => {
                <&str as sqlx::Encode<'q, DB>>::encode_by_ref(&&domain[..], buf)
            },
        }
    }

    fn encode(
        self,
        buf: &mut <DB as sqlx::Database>::ArgumentBuffer<'q>,
    ) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + Sync + Send>> {
        match self {
            Self::Ipv4(addr) => <Ipv4Addr as sqlx::Encode<'q, DB>>::encode(addr, buf),
            Self::Ipv6(addr) => <Ipv6Addr as sqlx::Encode<'q, DB>>::encode(addr, buf),
            Self::Domain(domain) => <String as sqlx::Encode<'q, DB>>::encode(domain, buf),
        }
    }

    fn produces(&self) -> Option<<DB as sqlx::Database>::TypeInfo> {
        match self {
            Self::Ipv4(addr) => <Ipv4Addr as sqlx::Encode<'q, DB>>::produces(addr),
            Self::Ipv6(addr) => <Ipv6Addr as sqlx::Encode<'q, DB>>::produces(addr),
            Self::Domain(domain) => <&str as sqlx::Encode<'q, DB>>::produces(&&domain[..]),
        }
    }

    fn size_hint(&self) -> usize {
        match self {
            Self::Ipv4(addr) => <Ipv4Addr as sqlx::Encode<'q, DB>>::size_hint(addr),
            Self::Ipv6(addr) => <Ipv6Addr as sqlx::Encode<'q, DB>>::size_hint(addr),
            Self::Domain(domain) => <&str as sqlx::Encode<'q, DB>>::size_hint(&&domain[..]),
        }
    }
}

impl<'r, DB: sqlx::Database> sqlx::Decode<'r, DB> for ServerHost
where
    &'r str: sqlx::Decode<'r, DB>,
{
    fn decode(
        value: <DB as sqlx::Database>::ValueRef<'r>,
    ) -> Result<Self, Box<dyn std::error::Error + Sync + Send>> {
        let value = <&'r str as sqlx::Decode<'r, DB>>::decode(value)?;

        if let Ok(addr) = value.parse::<Ipv6Addr>() {
            return Ok(Self::Ipv6(addr));
        }

        Ok(match url::Host::parse(value)? {
            url::Host::Ipv4(addr) => Self::Ipv4(addr),
            url::Host::Ipv6(addr) => Self::Ipv6(addr),
            url::Host::Domain(domain) => Self::Domain(domain.to_owned()),
        })
    }
}

impl<'de> serde::Deserialize<'de> for ServerHost {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        url::Host::parse(&<String as serde::Deserialize<'de>>::deserialize(deserializer)?)
            .map_err(serde::de::Error::custom)
            .map(|host| match host {
                url::Host::Ipv4(addr) => Self::Ipv4(addr),
                url::Host::Ipv6(addr) => Self::Ipv6(addr),
                url::Host::Domain(domain) => Self::Domain(domain),
            })
    }
}

#[cfg(feature = "fake")]
impl fake::Dummy<fake::Faker> for ServerHost {
    fn dummy_with_rng<R: fake::rand::Rng + ?Sized>(faker: &fake::Faker, rng: &mut R) -> Self {
        use fake::Fake;

        if rng.r#gen() {
            Self::Ipv4(faker.fake())
        } else {
            Self::Ipv6(faker.fake())
        }
    }
}
