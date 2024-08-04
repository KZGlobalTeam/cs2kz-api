//! Unit tests.

use crate::Mode;

#[test]
fn try_from_u8()
{
	assert!(Mode::try_from(0_u8).is_err());

	assert_eq!(Mode::try_from(1_u8), Ok(Mode::Vanilla));
	assert_eq!(Mode::try_from(2_u8), Ok(Mode::Classic));

	assert!(Mode::try_from(3_u8).is_err());
}

#[test]
fn into_u8()
{
	assert_eq!(u8::from(Mode::Vanilla), 1);
	assert_eq!(u8::from(Mode::Classic), 2);
}

#[test]
fn parse_u8()
{
	assert_eq!("1".parse::<Mode>(), Ok(Mode::Vanilla));
	assert_eq!("2".parse::<Mode>(), Ok(Mode::Classic));
}

#[test]
fn parse_str()
{
	assert_eq!("vnl".parse::<Mode>(), Ok(Mode::Vanilla));
	assert_eq!("vaNilLa".parse::<Mode>(), Ok(Mode::Vanilla));
	assert_eq!("CKZ".parse::<Mode>(), Ok(Mode::Classic));
	assert_eq!("classic".parse::<Mode>(), Ok(Mode::Classic));
}
