use std::num::ParseIntError;
use std::str::{self, FromStr};
use std::{array, fmt};

use serde::de::{self, Deserialize, Deserializer};
use serde::ser::{Serialize, Serializer};

const RAW_LEN: usize = 20;
const STR_LEN: usize = RAW_LEN * 2;

#[derive(Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "fake", derive(fake::Dummy))]
pub struct GitRevision {
    bytes: [u8; RAW_LEN],
}

#[derive(Debug, Display, Error)]
pub enum ParseGitRevisionError {
    #[display("invalid length; expected {STR_LEN} but got {got}")]
    InvalidLength { got: usize },

    #[display("failed to parse hex digit: {_0}")]
    ParseHexDigit(ParseIntError),
}

impl fmt::Debug for GitRevision {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_tuple("GitRevision")
            .field(&crate::fmt::DisplayAsDebug(self))
            .finish()
    }
}

impl fmt::Display for GitRevision {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.bytes
            .iter()
            .try_for_each(|byte| write!(fmt, "{byte:02x}"))
    }
}

impl FromStr for GitRevision {
    type Err = ParseGitRevisionError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.len() != STR_LEN {
            return Err(ParseGitRevisionError::InvalidLength { got: value.len() });
        }

        Ok(Self {
            bytes: array::try_from_fn(|idx| {
                let substr = value
                    .get(idx * 2..(idx + 1) * 2)
                    .expect("we checked the input's length");

                u8::from_str_radix(substr, 16).map_err(ParseGitRevisionError::ParseHexDigit)
            })?,
        })
    }
}

impl Serialize for GitRevision {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            format_args!("{self}").serialize(serializer)
        } else {
            self.bytes.serialize(serializer)
        }
    }
}

impl<'de> Deserialize<'de> for GitRevision {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        if !deserializer.is_human_readable() {
            return <[u8; RAW_LEN]>::deserialize(deserializer).map(|bytes| Self { bytes });
        }

        String::deserialize(deserializer)?
            .parse::<Self>()
            .map_err(|err| match err {
                ParseGitRevisionError::InvalidLength { got } => {
                    de::Error::invalid_length(got, &"40 hex characters")
                },
                ParseGitRevisionError::ParseHexDigit(error) => de::Error::custom(error),
            })
    }
}

crate::database::impl_traits!(GitRevision as [u8] => {
    fn encode<'a>(self, out: &'a [u8]) {
        out = &self.bytes[..];
    }

    fn decode<'a>(bytes: &'a [u8]) -> Result<Self, BoxError> {
        <[u8; RAW_LEN]>::try_from(bytes)
            .map(|bytes| Self { bytes })
            .map_err(Into::into)
    }
});
