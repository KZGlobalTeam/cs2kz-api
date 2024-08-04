//! [`Iterator`]s for [`Styles`].

use std::marker::PhantomData;

use super::Styles;

/// An iterator over the multiple styles contained in a [`Styles`] bitflags set.
#[derive(Debug)]
pub struct Iter<Item: ?Sized = ()>
{
	/// The bits we're iterating over.
	bits: u32,

	/// Type state for which item type we want to yield.
	_marker: PhantomData<Item>,
}

impl<Item: ?Sized> Clone for Iter<Item>
{
	fn clone(&self) -> Self
	{
		*self
	}
}

impl<Item: ?Sized> Copy for Iter<Item> {}

impl Iter
{
	/// Creates a new [`Iter`].
	pub const fn new(styles: Styles) -> Self
	{
		Self { bits: styles.bits(), _marker: PhantomData }
	}
}

impl<Item: ?Sized> Iter<Item>
{
	/// Switches the iterator to iterate over bits.
	pub const fn bits(self) -> Iter<u32>
	{
		Iter { bits: self.bits, _marker: PhantomData }
	}

	/// Switches the iterator to iterate over names.
	pub const fn names(self) -> Iter<str>
	{
		Iter { bits: self.bits, _marker: PhantomData }
	}
}

impl Iterator for Iter<u32>
{
	type Item = u32;

	fn next(&mut self) -> Option<Self::Item>
	{
		while self.bits != 0 {
			let lsb = 1 << self.bits.trailing_zeros();
			let styles = Styles::new_checked(lsb);

			self.bits &= !lsb;

			if let Some(Styles(styles)) = styles {
				return Some(styles);
			}
		}

		None
	}
}

impl Iterator for Iter<str>
{
	type Item = &'static str;

	fn next(&mut self) -> Option<Self::Item>
	{
		let mut bits = (*self).bits();

		bits.next().and_then(|item| {
			*self = bits.names();
			Styles(item).name()
		})
	}
}
