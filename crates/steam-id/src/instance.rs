const MASK: u64 =
    0b0000_0000_0000_1111_1111_1111_1111_1111_0000_0000_0000_0000_0000_0000_0000_0000_u64;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[derive(Debug, Display, Binary, LowerHex, UpperHex, Octal)]
pub struct Instance(u32);

impl Instance {
    pub const DEFAULT: Self = Self(1);

    pub const fn get(&self) -> u32 {
        self.0
    }

    pub(crate) const fn from_u64(value: u64) -> Self {
        Self(((value & MASK) >> 32) as u32)
    }
}
