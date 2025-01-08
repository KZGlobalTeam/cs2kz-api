use std::num::ParseIntError;
use std::str::FromStr;

const MASK: u64 = 0b0000000000000000000000000000000011111111111111111111111111111110u64;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[derive(Debug, Display, Binary, LowerHex, UpperHex, Octal)]
pub struct AccountNumber(u32);

#[derive(Debug, Display, Error, From)]
#[display("failed to parse SteamID account number: {reason}")]
#[from(forward)]
pub struct ParseAccountNumberError {
    reason: ParseAccountNumberErrorReason,
}

#[derive(Debug, Display, Error, From)]
pub enum ParseAccountNumberErrorReason {
    #[display("{_0}")]
    ParseInt(ParseIntError),

    #[display("value is too big for a valid account number")]
    TooBig,
}

impl AccountNumber {
    pub const fn new(value: u32) -> Option<Self> {
        if value <= (u32::MAX >> 1) {
            Some(Self(value))
        } else {
            None
        }
    }

    pub const fn get(&self) -> u32 {
        self.0
    }

    pub(crate) const fn from_u64(value: u64) -> Self {
        Self(((value & MASK) >> 1) as u32)
    }
}

impl FromStr for AccountNumber {
    type Err = ParseAccountNumberError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(Self::new(value.parse::<u32>()?).ok_or(ParseAccountNumberErrorReason::TooBig)?)
    }
}
