use std::num::ParseIntError;
use std::str::FromStr;
use std::{array, fmt, io};

use md5::{Digest, Md5};

const RAW_LEN: usize = 16;
const STR_LEN: usize = RAW_LEN * 2;

/// An MD5 hash
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "fake", derive(fake::Dummy))]
pub struct Checksum {
    #[debug("{self}")]
    bytes: [u8; RAW_LEN],
}

#[derive(Debug, Display, Error)]
pub enum ParseChecksumError {
    #[display("invalid length; expected {STR_LEN} but got {got}")]
    InvalidLength { got: usize },

    #[display("failed to parse hex digit: {_0}")]
    ParseHexDigit(ParseIntError),
}

impl Checksum {
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let mut hasher = Md5::new();
        hasher.update(bytes);

        Self { bytes: hasher.finalize().into() }
    }

    pub fn from_reader(reader: &mut impl io::Read) -> io::Result<Self> {
        let mut hasher = Md5::new();
        io::copy(reader, &mut hasher)?;

        Ok(Self { bytes: hasher.finalize().into() })
    }
}

impl fmt::Display for Checksum {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.bytes
            .iter()
            .try_for_each(|byte| write!(fmt, "{byte:02x}"))
    }
}

impl FromStr for Checksum {
    type Err = ParseChecksumError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.len() != STR_LEN {
            return Err(ParseChecksumError::InvalidLength { got: value.len() });
        }

        Ok(Self {
            bytes: array::try_from_fn(|idx| {
                let substr = value
                    .get(idx * 2..(idx + 1) * 2)
                    .expect("we checked the input's length");

                u8::from_str_radix(substr, 16).map_err(ParseChecksumError::ParseHexDigit)
            })?,
        })
    }
}

impl serde::Serialize for Checksum {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        format_args!("{self}").serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for Checksum {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        if !deserializer.is_human_readable() {
            return <[u8; RAW_LEN]>::deserialize(deserializer).map(|bytes| Self { bytes });
        }

        <String as serde::Deserialize<'de>>::deserialize(deserializer)?
            .parse::<Self>()
            .map_err(|err| match err {
                ParseChecksumError::InvalidLength { got } => {
                    serde::de::Error::invalid_length(got, &"32 hex characters")
                },
                ParseChecksumError::ParseHexDigit(error) => serde::de::Error::custom(error),
            })
    }
}

crate::database::impl_traits!(Checksum as [u8] => {
    fn encode<'a>(self, out: &'a [u8]) {
        out = &self.bytes[..];
    }

    fn decode<'a>(bytes: &'a [u8]) -> Result<Self, BoxError> {
        <[u8; 16]>::try_from(bytes)
            .map(|bytes| Self { bytes })
            .map_err(Into::into)
    }
});
