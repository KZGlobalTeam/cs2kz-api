use std::str::FromStr;

const MASK: u64 =
    0b0000_0000_1111_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_u64;

#[repr(u8)]
#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AccountType {
    Invalid = 0,
    Individual = 1,
    Multiseat = 2,
    GameServer = 3,
    AnonGameServer = 4,
    Pending = 5,
    ContentServer = 6,
    Clan = 7,
    Chat = 8,
    P2PSuperSeeder = 9,
    AnonUser = 10,
}

#[derive(Debug, Display, Error)]
#[display("invalid SteamID account type")]
pub struct ParseAccountTypeError {
    _priv: (),
}

impl AccountType {
    pub const fn identifier(&self) -> Option<u64> {
        match self {
            Self::Individual => Some(0x01_10_00_01_00_00_00_00),
            Self::Clan => Some(0x01_70_00_00_00_00_00_00),
            _ => None,
        }
    }

    pub const fn as_char(&self) -> char {
        match self {
            Self::Invalid => 'I',
            Self::Individual => 'U',
            Self::Multiseat => 'M',
            Self::GameServer => 'G',
            Self::AnonGameServer => 'A',
            Self::Pending => 'P',
            Self::ContentServer => 'C',
            Self::Clan => 'g',
            Self::Chat => 'T',
            Self::P2PSuperSeeder => '\0',
            Self::AnonUser => 'a',
        }
    }

    pub(crate) const fn from_u64(value: u64) -> Option<Self> {
        match (value & MASK) >> 52 {
            0 => Some(Self::Invalid),
            1 => Some(Self::Individual),
            2 => Some(Self::Multiseat),
            3 => Some(Self::GameServer),
            4 => Some(Self::AnonGameServer),
            5 => Some(Self::Pending),
            6 => Some(Self::ContentServer),
            7 => Some(Self::Clan),
            8 => Some(Self::Chat),
            9 => Some(Self::P2PSuperSeeder),
            10 => Some(Self::AnonUser),
            _ => None,
        }
    }
}

impl FromStr for AccountType {
    type Err = ParseAccountTypeError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "0" | "Invalid" | "invalid" | "I" | "i" => Ok(Self::Invalid),
            "1" | "Individual" | "individual" | "U" => Ok(Self::Individual),
            "2" | "Multiseat" | "multiseat" | "M" => Ok(Self::Multiseat),
            "3" | "GameServer" | "gameserver" | "G" => Ok(Self::GameServer),
            "4" | "AnonGameServer" | "anongameserver" | "A" => Ok(Self::AnonGameServer),
            "5" | "Pending" | "pending" | "P" => Ok(Self::Pending),
            "6" | "ContentServer" | "contentserver" | "C" => Ok(Self::ContentServer),
            "7" | "Clan" | "clan" | "g" => Ok(Self::Clan),
            "8" | "Chat" | "chat" | "T" | "L" | "c" => Ok(Self::Chat),
            "9" | "P2PSuperSeeder" | "p2psuperseeder" => Ok(Self::P2PSuperSeeder),
            "10" | "AnonUser" | "anonuser" | "a" => Ok(Self::AnonUser),
            _ => Err(ParseAccountTypeError { _priv: () }),
        }
    }
}
