//! [`Iterator`]s for [`Permissions`].

use std::marker::PhantomData;

use super::Permissions;

/// An iterator over the multiple permissions contained in a [`Permissions`]
/// bitflags set.
#[derive(Debug)]
pub struct Iter<Item: ?Sized = ()>
{
	/// The bits we're iterating over.
	bits: u64,

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
	pub const fn new(permissions: Permissions) -> Self
	{
		Self { bits: permissions.bits(), _marker: PhantomData }
	}
}

impl<Item: ?Sized> Iter<Item>
{
	/// Switches the iterator to iterate over bits.
	#[allow(private_interfaces)]
	pub const fn bits(self) -> Iter<u64>
	{
		Iter { bits: self.bits, _marker: PhantomData }
	}

	/// Switches the iterator to iterate over names.
	#[allow(private_interfaces)]
	pub const fn names(self) -> Iter<str>
	{
		Iter { bits: self.bits, _marker: PhantomData }
	}
}

impl Iterator for Iter<u64>
{
	type Item = u64;

	fn next(&mut self) -> Option<Self::Item>
	{
		while self.bits != 0 {
			let lsb = 1 << self.bits.trailing_zeros();
			let permissions = Permissions::new_checked(lsb);

			self.bits &= !lsb;

			if let Some(Permissions(permissions)) = permissions {
				return Some(permissions);
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
			Permissions(item).name()
		})
	}
}
