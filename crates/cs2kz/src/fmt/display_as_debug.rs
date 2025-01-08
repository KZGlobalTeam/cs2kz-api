use std::fmt;

/// A compatibility shim that implements [`fmt::Debug`] and [`fmt::Display`] in terms of
/// [`fmt::Display`].
pub struct DisplayAsDebug<'a, T: ?Sized>(pub &'a T);

impl<T: ?Sized + fmt::Display> fmt::Display for DisplayAsDebug<'_, T> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        <T as fmt::Display>::fmt(self.0, fmt)
    }
}

impl<T: ?Sized + fmt::Display> fmt::Debug for DisplayAsDebug<'_, T> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        <T as fmt::Display>::fmt(self.0, fmt)
    }
}
