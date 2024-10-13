//! Unit tests.

use crate::SteamID;

const ALPHAKEKS: SteamID = unsafe { SteamID::new_unchecked(76561198282622073_u64) };

#[test]
fn new()
{
	assert!(SteamID::new(76561198282622073_u64).is_some());

	assert!(SteamID::new(76561197960265728_u64).is_none());
	assert!(SteamID::new(76561197960265729_u64).is_some());

	assert!(SteamID::new(76561202255233023_u64).is_some());
	assert!(SteamID::new(76561202255233024_u64).is_none());
}

#[test]
fn try_from_u64()
{
	assert!(SteamID::try_from(76561198282622073_u64).is_ok());

	assert!(SteamID::try_from(76561197960265728_u64).is_err());
	assert!(SteamID::try_from(76561197960265729_u64).is_ok());

	assert!(SteamID::try_from(76561202255233023_u64).is_ok());
	assert!(SteamID::try_from(76561202255233024_u64).is_err());
}

#[test]
fn from_u32()
{
	assert_eq!(SteamID::from_u32(322356345), Some(ALPHAKEKS));

	assert!(SteamID::from_u32(0).is_none());
	assert!(SteamID::from_u32(1).is_some());

	assert!(SteamID::from_u32((super::MAX - super::MAGIC_OFFSET) as u32).is_some());
	assert!(SteamID::from_u32((super::MAX - super::MAGIC_OFFSET + 1) as u32).is_none());
}

#[test]
fn try_from_u32()
{
	assert_eq!(SteamID::try_from(322356345_u32), Ok(ALPHAKEKS));

	assert!(SteamID::try_from(0_u32).is_err());
	assert!(SteamID::try_from(1_u32).is_ok());

	assert!(SteamID::try_from((super::MAX - super::MAGIC_OFFSET) as u32).is_ok());
	assert!(SteamID::try_from((super::MAX - super::MAGIC_OFFSET + 1) as u32).is_err());
}

#[test]
fn parse_u64()
{
	assert!("76561198282622073".parse::<SteamID>().is_ok());

	assert!("76561197960265728".parse::<SteamID>().is_err());
	assert!("76561197960265729".parse::<SteamID>().is_ok());

	assert!("76561202255233023".parse::<SteamID>().is_ok());
	assert!("76561202255233024_u64".parse::<SteamID>().is_err());
}

#[test]
fn parse_u32()
{
	assert_eq!("322356345".parse::<SteamID>(), Ok(ALPHAKEKS));

	assert!("0".parse::<SteamID>().is_err());
	assert!("1".parse::<SteamID>().is_ok());

	assert!(((super::MAX - super::MAGIC_OFFSET) as u32)
		.to_string()
		.parse::<SteamID>()
		.is_ok());

	assert!(((super::MAX - super::MAGIC_OFFSET + 1) as u32)
		.to_string()
		.parse::<SteamID>()
		.is_err());
}

#[test]
fn parse_standard()
{
	assert_eq!("STEAM_0:1:161178172".parse::<SteamID>(), Ok(ALPHAKEKS));
	assert_eq!("STEAM_1:1:161178172".parse::<SteamID>(), Ok(ALPHAKEKS));
}

#[test]
fn parse_id3()
{
	assert_eq!("U:1:322356345".parse::<SteamID>(), Ok(ALPHAKEKS));
	assert_eq!("[U:1:322356345]".parse::<SteamID>(), Ok(ALPHAKEKS));

	assert!("U:1:322356345]".parse::<SteamID>().is_err());
	assert!("[U:1:322356345".parse::<SteamID>().is_err());
}
