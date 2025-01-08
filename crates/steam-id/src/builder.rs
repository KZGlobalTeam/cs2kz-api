//! A type-state builder for [`SteamId`]s.

use crate::{CommunityId, SteamId};

#[derive(Debug)]
pub struct Builder<
    Y = (),
    AccountNumber = (),
    Instance = crate::Instance,
    AccountType = crate::AccountType,
    Universe = crate::Universe,
> {
    y: Y,
    account_number: AccountNumber,
    instance: Instance,
    account_type: AccountType,
    universe: Universe,
}

impl Builder {
    pub const fn new() -> Self {
        Self {
            y: (),
            account_number: (),
            instance: crate::Instance::DEFAULT,
            account_type: crate::AccountType::Individual,
            universe: crate::Universe::Public,
        }
    }
}

impl<Y, AccountNumber, Instance, AccountType, Universe>
    Builder<Y, AccountNumber, Instance, AccountType, Universe>
// NOTE: without these bounds, the generics could potentially implement `Drop`, and we can't
// execute drop glue at compile time (the functions couldn't be `const`)
where
    Y: Copy,
    AccountNumber: Copy,
    Instance: Copy,
    AccountType: Copy,
    Universe: Copy,
{
    pub const fn y(
        self,
        y_bit_set: bool,
    ) -> Builder<bool, AccountNumber, Instance, AccountType, Universe> {
        Builder {
            y: y_bit_set,
            account_number: self.account_number,
            instance: self.instance,
            account_type: self.account_type,
            universe: self.universe,
        }
    }

    pub const fn account_number(
        self,
        account_number: crate::AccountNumber,
    ) -> Builder<Y, crate::AccountNumber, Instance, AccountType, Universe> {
        Builder {
            y: self.y,
            account_number,
            instance: self.instance,
            account_type: self.account_type,
            universe: self.universe,
        }
    }

    pub const fn instance(
        self,
        instance: crate::Instance,
    ) -> Builder<Y, AccountNumber, crate::Instance, AccountType, Universe> {
        Builder {
            y: self.y,
            account_number: self.account_number,
            instance,
            account_type: self.account_type,
            universe: self.universe,
        }
    }

    pub const fn account_type(
        self,
        account_type: crate::AccountType,
    ) -> Builder<Y, AccountNumber, Instance, crate::AccountType, Universe> {
        Builder {
            y: self.y,
            account_number: self.account_number,
            instance: self.instance,
            account_type,
            universe: self.universe,
        }
    }

    pub const fn universe(
        self,
        universe: crate::Universe,
    ) -> Builder<Y, AccountNumber, Instance, AccountType, crate::Universe> {
        Builder {
            y: self.y,
            account_number: self.account_number,
            instance: self.instance,
            account_type: self.account_type,
            universe,
        }
    }
}

impl Builder<bool, crate::AccountNumber, crate::Instance, crate::AccountType, crate::Universe> {
    pub const fn from_steam_id(steam_id: SteamId) -> Self {
        Self {
            y: steam_id.y_bit() == 1,
            account_number: steam_id.account_number(),
            instance: steam_id.instance(),
            account_type: steam_id.account_type(),
            universe: steam_id.universe(),
        }
    }

    pub const fn from_community_id(community_id: CommunityId) -> Self {
        Builder::new()
            .y(community_id.y_bit() == 1)
            .account_number(community_id.account_number())
    }

    pub const fn build(self) -> SteamId {
        let raw = ((self.universe as u8 as u64) << 56)
            | ((self.account_type as u8 as u64) << 52)
            | ((self.instance.get() as u64) << 32)
            | ((self.account_number.get() as u64) << 1)
            | (self.y as u8 as u64);

        unsafe { SteamId::from_u64_unchecked(raw) }
    }
}

impl Default for Builder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AccountNumber, AccountType, Instance, Universe};

    #[test]
    fn it_works() {
        let account_number = AccountNumber::new(161178172_u32).unwrap();
        let steam_id = SteamId::builder()
            .y(true)
            .account_number(account_number)
            .build();

        assert_eq!(steam_id.y_bit(), 1);
        assert_eq!(steam_id.account_number(), account_number);
        assert_eq!(steam_id.instance(), Instance::DEFAULT);
        assert_eq!(steam_id.account_type(), AccountType::Individual);
        assert_eq!(steam_id.universe(), Universe::Public);

        assert_eq!(steam_id, SteamId::from_u64(76561198282622073_u64).unwrap());
    }
}
