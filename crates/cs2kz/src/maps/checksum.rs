use std::{fmt, io};

use md5::{Digest, Md5};

/// The MD5 hash of a map's `.vpk` file.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "fake", derive(fake::Dummy))]
pub struct MapChecksum {
    #[debug("{self}")]
    bytes: [u8; 16],
}

impl MapChecksum {
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

impl fmt::Display for MapChecksum {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.bytes
            .iter()
            .try_for_each(|byte| write!(fmt, "{byte:02x}"))
    }
}

impl serde::Serialize for MapChecksum {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        format_args!("{self}").serialize(serializer)
    }
}

crate::database::impl_traits!(MapChecksum as [u8] => {
    fn encode<'a>(self, out: &'a [u8]) {
        out = &self.bytes[..];
    }

    fn decode<'a>(bytes: &'a [u8]) -> Result<Self, BoxError> {
        <[u8; 16]>::try_from(bytes)
            .map(|bytes| Self { bytes })
            .map_err(Into::into)
    }
});
