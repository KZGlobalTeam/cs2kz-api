use std::num::ParseIntError;
use std::str::FromStr;

use crate::{AccountNumber, SteamId};

#[derive(Clone, Copy, Deref, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[derive(Debug, Display, Binary, LowerHex, UpperHex, Octal)]
pub struct CommunityId(u32);

#[derive(Debug, Display, Error, From)]
#[display("failed to parse SteamID account number: {reason}")]
#[from(forward)]
pub struct ParseCommunityIdError {
    reason: ParseCommunityIdErrorReason,
}

#[derive(Debug, Display, Error, From)]
pub enum ParseCommunityIdErrorReason {
    #[display("{_0}")]
    ParseInt(ParseIntError),

    #[display("value is too big for a valid Steam community ID")]
    TooBig,
}

impl CommunityId {
    pub const fn new(value: u32) -> Option<Self> {
        match (if (value & 1) == 1 { value - 1 } else { value }).checked_div(2) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }

    pub const fn get(&self) -> u32 {
        self.0
    }

    pub const fn y_bit(&self) -> u64 {
        (self.0 & 1) as u64
    }

    pub const fn account_number(&self) -> AccountNumber {
        match AccountNumber::new(self.0 / 2) {
            Some(account_number) => account_number,
            None => panic!("BUG: CommunityID contains invalid account number"),
        }
    }

    pub(crate) const fn from_steam_id(steam_id: SteamId) -> Self {
        Self((steam_id.account_number().get() * 2) + (steam_id.y_bit() as u32))
    }
}

impl From<SteamId> for CommunityId {
    fn from(steam_id: SteamId) -> Self {
        Self::from_steam_id(steam_id)
    }
}

impl From<CommunityId> for SteamId {
    fn from(community_id: CommunityId) -> Self {
        Self::from_community_id(community_id)
    }
}

impl FromStr for CommunityId {
    type Err = ParseCommunityIdError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(Self::new(value.parse::<u32>()?).ok_or(ParseCommunityIdErrorReason::TooBig)?)
    }
}
