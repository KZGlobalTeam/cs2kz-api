//! This module contains the [`IsEmpty`] trait.

use std::collections::BTreeMap;

/// This trait abstracts over containers that can be "empty".
///
/// Examples of "containers" include [`Vec`], [`String`], and [`BTreeMap`].
/// All of these have `len()` / `is_empty()` methods, but there is no standard
/// way of abstracting over those methods.
///
/// This trait exists so that you can write functions that are generic over
/// containers, if they care about whether the container is empty.
///
/// The main application for this currently are functions in [`crate::serde`].
#[sealed]
pub trait IsEmpty
{
	/// Checks if the container is empty.
	fn is_empty(&self) -> bool;
}

#[sealed]
impl<T> IsEmpty for [T]
{
	fn is_empty(&self) -> bool
	{
		<Self>::is_empty(self)
	}
}

#[sealed]
impl<T> IsEmpty for Vec<T>
{
	fn is_empty(&self) -> bool
	{
		<[T]>::is_empty(&self[..])
	}
}

#[sealed]
impl<K, V> IsEmpty for BTreeMap<K, V>
{
	fn is_empty(&self) -> bool
	{
		<Self>::is_empty(self)
	}
}

#[sealed]
impl IsEmpty for str
{
	fn is_empty(&self) -> bool
	{
		<Self>::is_empty(self)
	}
}

#[sealed]
impl IsEmpty for String
{
	fn is_empty(&self) -> bool
	{
		<str>::is_empty(self.as_str())
	}
}
