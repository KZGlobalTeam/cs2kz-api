use std::error::Error;
use std::fmt::Debug;

use serde::Serialize;

/// A value that can be recorded in a [`Log`].
///
/// [`Log`]: super::Log
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum Value {
	Bool(bool),
	Int(i64),
	Float(f64),
	String(String),
}

macro_rules! from {
	($ty:ty, $variant:ident) => {
		impl From<$ty> for Value {
			fn from(value: $ty) -> Self {
				Self::$variant(value)
			}
		}
	};

	($ty:ty, | $value:pat_param | $impl:block) => {
		impl From<$ty> for Value {
			fn from($value: $ty) -> Self {
				$impl
			}
		}
	};
}

from!(f64, Float);
from!(i64, Int);
from!(u64, |value| { Self::Int(value as _) });
from!(i128, |value| { Self::Int(value as _) });
from!(u128, |value| { Self::Int(value as _) });
from!(bool, Bool);
from!(&str, |value| { Self::String(value.to_owned()) });
from!(&dyn Debug, |value| { Self::String(format!("{value:?}")) });
from!(&dyn Error, |value| { Self::String(format!("{value}")) });
