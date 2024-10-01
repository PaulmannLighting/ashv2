use std::num::NonZero;
use std::ops::{Add, AddAssign};

const MASK: u8 = 0b0000_0111;
const NON_ZERO_MASK: u8 = 0b0000_1000;

/// A three bit number.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct WrappingU3(NonZero<u8>);

impl WrappingU3 {
    /// Creates a new three bit number.
    #[must_use]
    pub const fn from_u8_lossy(n: u8) -> Self {
        Self(nonzero_lossy(n))
    }

    /// Returns the number as an u8.
    #[must_use]
    pub const fn as_u8(self) -> u8 {
        self.0.get() ^ NON_ZERO_MASK
    }
}

impl Add<u8> for WrappingU3 {
    type Output = Self;

    fn add(self, rhs: u8) -> Self::Output {
        Self::from_u8_lossy(self.as_u8().wrapping_add(rhs))
    }
}

impl AddAssign<u8> for WrappingU3 {
    fn add_assign(&mut self, rhs: u8) {
        self.0 = nonzero_lossy(self.as_u8().wrapping_add(rhs));
    }
}

impl From<WrappingU3> for u8 {
    fn from(value: WrappingU3) -> Self {
        value.as_u8()
    }
}

impl TryFrom<u8> for WrappingU3 {
    type Error = u8;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        let u3 = Self::from_u8_lossy(value);

        if u3.as_u8() == value {
            Ok(u3)
        } else {
            Err(value)
        }
    }
}

/// Creates a non-zero three bit number stored in four bits
/// where the fourth bit is `1` making the number non-zero.
const fn nonzero_lossy(n: u8) -> NonZero<u8> {
    #[allow(unsafe_code)]
    // SAFETY: We create a three bit number by applying `MASK` to `n`.
    // Finally, we XOR the result with `NON_ZERO_MASK`, which makes the number non-zero.
    unsafe {
        NonZero::new_unchecked(n & MASK | NON_ZERO_MASK)
    }
}

#[cfg(test)]
mod tests {
    use super::WrappingU3;

    #[test]
    fn test_new() {
        for n in u8::MIN..=u8::MAX {
            let number = WrappingU3::from_u8_lossy(n);
            assert_eq!(u8::from(number), n % 8);
        }
    }

    #[test]
    fn test_as_u8() {
        for n in u8::MIN..=u8::MAX {
            let number = WrappingU3::from_u8_lossy(n);
            assert_eq!(number.as_u8(), n % 8);
        }
    }

    #[test]
    fn test_add() {
        for n in 0..=u8::MAX {
            for rhs in 0..=u8::MAX {
                let number = WrappingU3::from_u8_lossy(n) + rhs;
                assert_eq!(u8::from(number), n.wrapping_add(rhs) % 8);
            }
        }
    }

    #[test]
    fn test_add_assign() {
        for n in 0..=u8::MAX {
            for rhs in 0..=u8::MAX {
                let mut number = WrappingU3::from_u8_lossy(n);
                number += rhs;
                assert_eq!(u8::from(number), n.wrapping_add(rhs) % 8);
            }
        }
    }
}
