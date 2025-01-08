/* Copyright (C) 2024  AlphaKeks <alphakeks@dawn.sh>
 *
 * This library is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This library is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this repository.  If not, see <https://www.gnu.org/licenses/>.
 */

//! An implementation of [Valve's SteamID].
//!
//! [Valve's SteamID]: https://developer.valvesoftware.com/wiki/SteamID

#[macro_use]
extern crate derive_more;

use std::borrow::Borrow;
use std::str::FromStr;
use std::{cmp, fmt};

mod account_number;
pub use account_number::{AccountNumber, ParseAccountNumberError};

mod instance;
pub use instance::Instance;

mod account_type;
pub use account_type::{AccountType, ParseAccountTypeError};

mod universe;
pub use universe::{ParseUniverseError, Universe};

mod community_id;
pub use community_id::{CommunityId, ParseCommunityIdError};

mod builder;
pub use builder::Builder;

#[cfg(feature = "serde")]
pub mod serde;

/// A SteamID.
#[repr(transparent)]
#[derive(Clone, Copy, Deref, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[derive(Binary, LowerHex, UpperHex, Octal)]
pub struct SteamId(u64);

#[derive(Debug, Display, Error)]
pub enum InvalidSteamId {
    #[display("invalid account type bits")]
    InvalidAccountType,

    #[display("invalid universe")]
    InvalidUniverse,
}

#[derive(Debug, Display, Error, From)]
#[display("failed to parse SteamID: {reason}")]
#[from(forward)]
pub struct ParseStandardError<'a> {
    reason: ParseStandardErrorReason<'a>,
}

#[derive(Debug, Display, Error, From)]
pub enum ParseStandardErrorReason<'a> {
    #[display("missing `STEAM_` prefix")]
    MissingPrefix,

    #[display("missing universe ('X') segment")]
    MissingUniverse,

    #[display("invalid universe ('X') segment: {_0}")]
    InvalidUniverse(ParseUniverseError),

    #[display("missing 'Y' segment")]
    MissingY,

    #[display("invalid 'Y' segment: `{actual}`")]
    #[from(ignore)]
    InvalidY {
        #[error(ignore)]
        actual: &'a str,
    },

    #[display("missing account number segment")]
    MissingAccountNumber,

    #[display("invalid account number segment: {_0}")]
    InvalidAccountNumber(ParseAccountNumberError),
}

#[derive(Debug, Display, Error, From)]
#[display("failed to parse Steam community ID: {reason}")]
#[from(forward)]
pub struct ParseCommunityError<'a> {
    reason: ParseCommunityErrorReason<'a>,
}

#[derive(Debug, Display, Error, From)]
pub enum ParseCommunityErrorReason<'a> {
    #[display("inconsistent brackets")]
    InconsistentBrackets,

    #[display("missing account type segment")]
    MissingAccountType,

    #[display("invalid account type segment")]
    InvalidAccountType(ParseAccountTypeError),

    #[display("missing '1' segment")]
    MissingOne,

    #[display("second segment should have been '1' but was `{actual}`")]
    #[from(ignore)]
    SecondSegmentNotOne {
        #[error(ignore)]
        actual: &'a str,
    },

    #[display("missing ID segment")]
    MissingId,

    #[display("invalid ID segment: {_0}")]
    InvalidId(ParseCommunityIdError),

    #[display("invalid account number")]
    InvalidAccountNumber,
}

#[derive(Debug, Display, Error)]
#[display("failed to parse SteamID")]
pub struct ParseSteamIdError {
    raw_error: Option<InvalidSteamId>,
}

impl SteamId {
    pub const fn as_u64(&self) -> u64 {
        self.0
    }

    #[doc(alias = "x")]
    pub const fn universe(&self) -> Universe {
        match Universe::from_u64(self.0) {
            Some(universe) => universe,
            None => panic!("BUG: SteamID contains invalid universe bits"),
        }
    }

    pub const fn y_bit(&self) -> u64 {
        self.0 & 1
    }

    #[doc(alias = "z")]
    pub const fn account_number(&self) -> AccountNumber {
        AccountNumber::from_u64(self.0)
    }

    pub const fn instance(&self) -> Instance {
        Instance::from_u64(self.0)
    }

    pub const fn account_type(&self) -> AccountType {
        match AccountType::from_u64(self.0) {
            Some(account_type) => account_type,
            None => panic!("BUG: SteamID contains invalid account type bits"),
        }
    }

    pub const fn as_community_id(self) -> CommunityId {
        CommunityId::from_steam_id(self)
    }

    pub const fn from_u64(value: u64) -> Result<Self, InvalidSteamId> {
        let y = value & 1 == 1;
        let account_number = AccountNumber::from_u64(value);
        let instance = Instance::from_u64(value);
        let Some(account_type) = AccountType::from_u64(value) else {
            return Err(InvalidSteamId::InvalidAccountType);
        };
        let Some(universe) = Universe::from_u64(value) else {
            return Err(InvalidSteamId::InvalidUniverse);
        };

        Ok(Builder::new()
            .y(y)
            .account_number(account_number)
            .instance(instance)
            .account_type(account_type)
            .universe(universe)
            .build())
    }

    /// # Safety
    ///
    /// The caller must ensure that `value` is a valid SteamId.
    pub const unsafe fn from_u64_unchecked(value: u64) -> Self {
        Self(value)
    }

    pub const fn from_community_id(community_id: CommunityId) -> Self {
        Self::builder()
            .y(community_id.y_bit() == 1)
            .account_number(community_id.account_number())
            .build()
    }

    pub const fn builder() -> Builder {
        Builder::new()
    }

    /// Parses the "standard" `STEAM_X:Y:Z` format into a [`SteamId`].
    pub fn parse_standard(value: &str) -> Result<Self, ParseStandardError<'_>> {
        let mut segments = value
            .strip_prefix("STEAM_")
            .ok_or(ParseStandardErrorReason::MissingPrefix)?
            .splitn(3, ':');

        let universe = segments
            .next()
            .ok_or(ParseStandardErrorReason::MissingUniverse)?
            .parse::<Universe>()?;

        let y = match segments.next() {
            Some("0") => Ok(0),
            Some("1") => Ok(1),
            Some("") | None => Err(ParseStandardErrorReason::MissingY),
            Some(actual) => Err(ParseStandardErrorReason::InvalidY { actual }),
        }?;

        let account_number = segments
            .next()
            .ok_or(ParseStandardErrorReason::MissingAccountNumber)?
            .parse::<AccountNumber>()?;

        Ok(Self::builder()
            .y(y == 1)
            .account_number(account_number)
            .universe(universe)
            .build())
    }

    /// Parses the "Community ID" format `[U:1:XXXXXXXXXX]`.
    ///
    /// The brackets are optional, but if one of them is present, the other one must also be there.
    pub fn parse_community(value: &str) -> Result<Self, ParseCommunityError<'_>> {
        let mut segments = match (value.starts_with('['), value.ends_with(']')) {
            (false, false) => Ok(value),
            (true, true) => Ok(&value[1..value.len() - 2]),
            (true, false) | (false, true) => Err(ParseCommunityErrorReason::InconsistentBrackets),
        }?
        .splitn(3, ':');

        let account_type = segments
            .next()
            .ok_or(ParseCommunityErrorReason::MissingAccountType)?
            .parse::<AccountType>()?;

        match segments.next() {
            Some("1") => Ok(()),
            Some("") | None => Err(ParseCommunityErrorReason::MissingOne),
            Some(actual) => Err(ParseCommunityErrorReason::SecondSegmentNotOne { actual }),
        }?;

        let community_id = segments
            .next()
            .ok_or(ParseCommunityErrorReason::MissingId)?
            .parse::<CommunityId>()?;

        Ok(Builder::from_community_id(community_id)
            .account_type(account_type)
            .build())
    }
}

impl fmt::Debug for SteamId {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        if fmt.alternate() {
            fmt.debug_struct("SteamId")
                .field("universe", &self.universe())
                .field("account_type", &self.account_type())
                .field("instance", &self.instance())
                .field("account_number", &self.account_number())
                .field("Y", &self.y_bit())
                .finish()
        } else {
            <Self as fmt::Display>::fmt(self, fmt)
        }
    }
}

impl fmt::Display for SteamId {
    /// By default SteamIDs will be displayed using the standard `STEAM_X:Y:Z` format.
    ///
    /// If the `#` sigil is included in the format string, it will be formatted as a "Community ID"
    /// instead.
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        if fmt.alternate() {
            write!(fmt, "[{}:1:{}]", self.account_type().as_char(), self.as_community_id())
        } else {
            write!(
                fmt,
                "STEAM_{}:{}:{}",
                self.universe() as u8,
                self.y_bit(),
                self.account_number()
            )
        }
    }
}

impl Borrow<u64> for SteamId {
    fn borrow(&self) -> &u64 {
        &self.0
    }
}

impl PartialEq<u64> for SteamId {
    fn eq(&self, rhs: &u64) -> bool {
        self.0 == *rhs
    }
}

impl PartialEq<SteamId> for u64 {
    fn eq(&self, rhs: &SteamId) -> bool {
        *self == rhs.0
    }
}

impl PartialEq<CommunityId> for SteamId {
    fn eq(&self, rhs: &CommunityId) -> bool {
        CommunityId::eq(&self.as_community_id(), rhs)
    }
}

impl PartialEq<SteamId> for CommunityId {
    fn eq(&self, rhs: &SteamId) -> bool {
        CommunityId::eq(self, &rhs.as_community_id())
    }
}

impl PartialOrd<u64> for SteamId {
    fn partial_cmp(&self, rhs: &u64) -> Option<cmp::Ordering> {
        u64::partial_cmp(&self.0, rhs)
    }
}

impl PartialOrd<SteamId> for u64 {
    fn partial_cmp(&self, rhs: &SteamId) -> Option<cmp::Ordering> {
        u64::partial_cmp(self, &rhs.0)
    }
}

impl PartialOrd<CommunityId> for SteamId {
    fn partial_cmp(&self, rhs: &CommunityId) -> Option<cmp::Ordering> {
        CommunityId::partial_cmp(&self.as_community_id(), rhs)
    }
}

impl PartialOrd<SteamId> for CommunityId {
    fn partial_cmp(&self, rhs: &SteamId) -> Option<cmp::Ordering> {
        CommunityId::partial_cmp(self, &rhs.as_community_id())
    }
}

impl TryFrom<u64> for SteamId {
    type Error = InvalidSteamId;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        Self::from_u64(value)
    }
}

/// Parsing is done on a best-effort basis.
///
/// All known formats will be attempted before returning an error. If you expect a particular
/// format, use one of the `SteamID::parse_*` constructors.
impl FromStr for SteamId {
    type Err = ParseSteamIdError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if let Ok(raw) = value.parse::<u64>() {
            Self::from_u64(raw).map_err(|err| ParseSteamIdError { raw_error: Some(err) })
        } else if let Ok(steam_id) = Self::parse_standard(value) {
            Ok(steam_id)
        } else if let Ok(steam_id) = Self::parse_community(value) {
            Ok(steam_id)
        } else {
            Err(ParseSteamIdError { raw_error: None })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display() {
        let steam_id = SteamId::from_u64(76561198282622073_u64).unwrap();
        assert_eq!(format!("{steam_id}"), "STEAM_1:1:161178172");
    }

    #[test]
    fn display_alternate() {
        let steam_id = SteamId::from_u64(76561198282622073_u64).unwrap();
        assert_eq!(format!("{steam_id:#}"), "[U:1:322356345]");
    }

    #[test]
    fn parse() {
        let steam_id = SteamId::from_u64(76561198282622073_u64).unwrap();
        assert_eq!("76561198282622073".parse::<SteamId>().unwrap(), steam_id);
    }
}
