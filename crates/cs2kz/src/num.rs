pub trait AsF64 {
    fn as_f64(&self) -> f64;
}

impl<T: AsF64> AsF64 for &T {
    fn as_f64(&self) -> f64 {
        T::as_f64(*self)
    }
}

impl AsF64 for f64 {
    fn as_f64(&self) -> f64 {
        *self
    }
}
