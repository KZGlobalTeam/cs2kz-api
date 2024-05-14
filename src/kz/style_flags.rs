//! This module contains a bitflag type for storing combined ["styles"].
//!
//! ["styles"]: cs2kz::Style

use cs2kz::Style;

crate::bitflags::bitflags! {
	/// Bitfield for holding style information.
	///
	/// See [`cs2kz::Style`].
	pub StyleFlags as u32 {
		NORMAL = { 1 << 0, "normal" };
		BACKWARDS = { 1 << 1, "auto_bhop" };
	}

	iter: StyleFlagsIter
}

impl FromIterator<Style> for StyleFlags {
	fn from_iter<I>(iter: I) -> Self
	where
		I: IntoIterator<Item = Style>,
	{
		iter.into_iter()
			.map(|style| StyleFlags::new(u8::from(style).into()))
			.fold(StyleFlags::NONE, |acc, curr| (acc | curr))
	}
}
