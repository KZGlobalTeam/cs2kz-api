use std::str::FromStr;
use std::{fmt, ops};

use serde::de::{self, Deserialize, Deserializer};
use serde::ser::{Serialize, SerializeSeq, Serializer};

const USER_PERMISSIONS: u64 = 0b_0001;
const SERVERS: u64 = 0b0010;
const MAP_POOL: u64 = 0b0100;
const PLAYER_BANS: u64 = 0b1000;

#[repr(u64)]
#[derive(Debug, Clone, Copy)]
pub enum Permission {
    UserPermissions = USER_PERMISSIONS,
    Servers = SERVERS,
    MapPool = MAP_POOL,
    PlayerBans = PLAYER_BANS,
}

#[derive(Debug, Default, Clone, Copy, sqlx::Type)]
#[sqlx(transparent)]
pub struct Permissions(u64);

#[derive(Debug, Clone)]
pub struct PermissionIter {
    bits: u64,
}

#[derive(Debug, Display, Error)]
#[display("unknown permission")]
pub struct UnknownPermission {
    _priv: (),
}

impl Permission {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::UserPermissions => "user-permissions",
            Self::Servers => "servers",
            Self::MapPool => "map-pool",
            Self::PlayerBans => "player-bans",
        }
    }
}

impl FromStr for Permission {
    type Err = UnknownPermission;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "user-permissions" => Ok(Self::UserPermissions),
            "servers" => Ok(Self::Servers),
            "map-pool" => Ok(Self::MapPool),
            "player-bans" => Ok(Self::PlayerBans),
            _ => Err(UnknownPermission { _priv: () }),
        }
    }
}

impl TryFrom<u64> for Permission {
    type Error = UnknownPermission;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        match value {
            USER_PERMISSIONS => Ok(Self::UserPermissions),
            SERVERS => Ok(Self::Servers),
            MAP_POOL => Ok(Self::MapPool),
            PLAYER_BANS => Ok(Self::PlayerBans),
            _ => Err(UnknownPermission { _priv: () }),
        }
    }
}

impl ops::BitAnd for Permission {
    type Output = Permissions;

    fn bitand(self, rhs: Permission) -> Self::Output {
        Permissions(self as u64 & rhs as u64)
    }
}

impl ops::BitAnd<Permissions> for Permission {
    type Output = Permissions;

    fn bitand(self, rhs: Permissions) -> Self::Output {
        Permissions(self as u64 & rhs.0)
    }
}

impl ops::BitOr for Permission {
    type Output = Permissions;

    fn bitor(self, rhs: Permission) -> Self::Output {
        Permissions(self as u64 | rhs as u64)
    }
}

impl ops::BitOr<Permissions> for Permission {
    type Output = Permissions;

    fn bitor(self, rhs: Permissions) -> Self::Output {
        Permissions(self as u64 | rhs.0)
    }
}

impl ops::BitXor for Permission {
    type Output = Permissions;

    fn bitxor(self, rhs: Permission) -> Self::Output {
        Permissions(self as u64 ^ rhs as u64)
    }
}

impl ops::BitXor<Permissions> for Permission {
    type Output = Permissions;

    fn bitxor(self, rhs: Permissions) -> Self::Output {
        Permissions(self as u64 ^ rhs.0)
    }
}

impl Permissions {
    pub const fn none() -> Self {
        Self(0)
    }

    pub fn contains(self, other: impl Into<Permissions>) -> bool {
        let other = Into::<Permissions>::into(other);
        (self.0 & other.0) == other.0
    }

    pub fn count(&self) -> usize {
        self.0.count_ones() as usize
    }

    pub fn iter(&self) -> PermissionIter {
        PermissionIter { bits: self.0 }
    }
}

impl Serialize for Permission {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.as_str().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Permission {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct PermissionVisitor;

        impl de::Visitor<'_> for PermissionVisitor {
            type Value = Permission;

            fn expecting(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(fmt, "a permission")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                value
                    .parse()
                    .map_err(|_| E::invalid_value(de::Unexpected::Str(value), &self))
            }
        }

        deserializer.deserialize_str(PermissionVisitor)
    }
}

impl From<Permission> for Permissions {
    fn from(permission: Permission) -> Self {
        Self(permission as u64)
    }
}

impl IntoIterator for Permissions {
    type Item = Permission;
    type IntoIter = PermissionIter;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl IntoIterator for &Permissions {
    type Item = Permission;
    type IntoIter = PermissionIter;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl Serialize for Permissions {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut serializer = serializer.serialize_seq(Some(self.count()))?;

        for permission in self {
            serializer.serialize_element(&permission)?;
        }

        serializer.end()
    }
}

impl<'de> Deserialize<'de> for Permissions {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct PermissionsVisitor;

        impl<'de> de::Visitor<'de> for PermissionsVisitor {
            type Value = Permissions;

            fn expecting(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(fmt, "a list of permissions")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let mut permissions = Permissions::none();

                while let Some(permission) = seq.next_element::<Permission>()? {
                    permissions |= permission;
                }

                Ok(permissions)
            }
        }

        deserializer.deserialize_seq(PermissionsVisitor)
    }
}

impl ops::BitAnd for Permissions {
    type Output = Permissions;

    fn bitand(self, rhs: Permissions) -> Self::Output {
        Permissions(self.0 & rhs.0)
    }
}

impl ops::BitAndAssign for Permissions {
    fn bitand_assign(&mut self, rhs: Permissions) {
        self.0 &= rhs.0;
    }
}

impl ops::BitAnd<Permission> for Permissions {
    type Output = Permissions;

    fn bitand(self, rhs: Permission) -> Self::Output {
        Permissions(self.0 & rhs as u64)
    }
}

impl ops::BitAndAssign<Permission> for Permissions {
    fn bitand_assign(&mut self, rhs: Permission) {
        self.0 &= rhs as u64;
    }
}

impl ops::BitOr for Permissions {
    type Output = Permissions;

    fn bitor(self, rhs: Permissions) -> Self::Output {
        Permissions(self.0 | rhs.0)
    }
}

impl ops::BitOrAssign for Permissions {
    fn bitor_assign(&mut self, rhs: Permissions) {
        self.0 |= rhs.0;
    }
}

impl ops::BitOr<Permission> for Permissions {
    type Output = Permissions;

    fn bitor(self, rhs: Permission) -> Self::Output {
        Permissions(self.0 | rhs as u64)
    }
}

impl ops::BitOrAssign<Permission> for Permissions {
    fn bitor_assign(&mut self, rhs: Permission) {
        self.0 |= rhs as u64;
    }
}

impl ops::BitXor for Permissions {
    type Output = Permissions;

    fn bitxor(self, rhs: Permissions) -> Self::Output {
        Permissions(self.0 ^ rhs.0)
    }
}

impl ops::BitXorAssign for Permissions {
    fn bitxor_assign(&mut self, rhs: Permissions) {
        self.0 ^= rhs.0;
    }
}

impl ops::BitXor<Permission> for Permissions {
    type Output = Permissions;

    fn bitxor(self, rhs: Permission) -> Self::Output {
        Permissions(self.0 ^ rhs as u64)
    }
}

impl ops::BitXorAssign<Permission> for Permissions {
    fn bitxor_assign(&mut self, rhs: Permission) {
        self.0 ^= rhs as u64;
    }
}

impl Iterator for PermissionIter {
    type Item = Permission;

    fn next(&mut self) -> Option<Self::Item> {
        if self.bits == 0 {
            return None;
        }

        let next_bit = 1 << self.bits.trailing_zeros();
        self.bits &= !next_bit;

        Some(Permission::try_from(next_bit).expect("invalid permission bit in `PermissionIter`"))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let count = self.bits.count_ones() as usize;
        (count, Some(count))
    }
}

impl ExactSizeIterator for PermissionIter {}
