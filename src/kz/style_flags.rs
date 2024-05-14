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
		BACKWARDS = { 1 << 1, "backwards" };
		SIDEWAYS = { 1 << 2, "sideways" };
		HALF_SIDEWAYS = { 1 << 3, "half_sideways" };
		W_ONLY = { 1 << 4, "w_only" };
		LOW_GRAVITY = { 1 << 5, "low_gravity" };
		HIGH_GRAVITY = { 1 << 6, "high_gravity" };
		NO_PRESTRAFE = { 1 << 7, "no_prestrafe" };
		NEGEV = { 1 << 8, "negev" };
		ICE = { 1 << 9, "ice" };
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
