use std::str::FromStr;

#[repr(u8)]
#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Universe {
    Individual = 0,
    Public = 1,
    Beta = 2,
    Internal = 3,
    Dev = 4,
    RC = 5,
}

#[derive(Debug, Display, Error)]
#[display("invalid SteamID universe")]
pub struct ParseUniverseError {
    #[error(ignore)]
    _priv: (),
}

impl Universe {
    pub(crate) const fn from_u64(value: u64) -> Option<Self> {
        match value.to_be_bytes()[0] {
            0 => Some(Self::Individual),
            1 => Some(Self::Public),
            2 => Some(Self::Beta),
            3 => Some(Self::Internal),
            4 => Some(Self::Dev),
            5 => Some(Self::RC),
            _ => None,
        }
    }
}

impl FromStr for Universe {
    type Err = ParseUniverseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "0" | "individual" | "Individual" => Ok(Self::Individual),
            "1" | "Public" | "public" => Ok(Self::Public),
            "2" | "Beta" | "beta" => Ok(Self::Beta),
            "3" | "Internal" | "internal" => Ok(Self::Internal),
            "4" | "Dev" | "dev" => Ok(Self::Dev),
            "5" | "RC" | "rc" => Ok(Self::RC),
            _ => Err(ParseUniverseError { _priv: () }),
        }
    }
}
