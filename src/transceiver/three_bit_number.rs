use std::num::NonZero;
use std::ops::{Add, AddAssign};

const MASK: u8 = 0b0000_0111;

/// An optional three bit number.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct ThreeBitNumber(NonZero<u8>);

impl ThreeBitNumber {
    /// Creates a new optional three bit number.
    #[must_use]
    pub const fn from_u8_lossy(n: u8) -> Self {
        Self(shifted_nonzero_three_bits_lossy(n))
    }

    /// Returns the number as an u8.
    #[must_use]
    pub const fn as_u8(self) -> u8 {
        self.0.get() >> 1
    }
}

impl Add<u8> for ThreeBitNumber {
    type Output = Self;

    fn add(self, rhs: u8) -> Self::Output {
        Self::from_u8_lossy(self.as_u8().wrapping_add(rhs))
    }
}

impl AddAssign<u8> for ThreeBitNumber {
    fn add_assign(&mut self, rhs: u8) {
        self.0 = shifted_nonzero_three_bits_lossy(self.as_u8().wrapping_add(rhs));
    }
}

impl From<ThreeBitNumber> for u8 {
    fn from(value: ThreeBitNumber) -> Self {
        value.as_u8()
    }
}

const fn shifted_nonzero_three_bits_lossy(n: u8) -> NonZero<u8> {
    #[allow(unsafe_code)]
    // SAFETY: We create a three bit number by applying `MASK` to `n`.
    // Then we shift that number to the left by one.
    // Finally, we OR the result with 1, which makes the number non-zero.
    unsafe {
        NonZero::new_unchecked(((n & MASK) << 1) | 1)
    }
}

#[cfg(test)]
mod tests {
    use super::ThreeBitNumber;

    #[test]
    fn test_new() {
        for n in u8::MIN..=u8::MAX {
            let number = ThreeBitNumber::from_u8_lossy(n);
            assert_eq!(u8::from(number), n % 8);
        }
    }

    #[test]
    fn test_as_u8() {
        for n in u8::MIN..=u8::MAX {
            let number = ThreeBitNumber::from_u8_lossy(n);
            assert_eq!(number.as_u8(), n % 8);
        }
    }

    #[test]
    fn test_add() {
        for n in 0..=u8::MAX {
            for rhs in 0..=u8::MAX {
                let number = ThreeBitNumber::from_u8_lossy(n) + rhs;
                assert_eq!(u8::from(number), n.wrapping_add(rhs) % 8);
            }
        }
    }

    #[test]
    fn test_add_assign() {
        for n in 0..=u8::MAX {
            for rhs in 0..=u8::MAX {
                let mut number = ThreeBitNumber::from_u8_lossy(n);
                number += rhs;
                assert_eq!(u8::from(number), n.wrapping_add(rhs) % 8);
            }
        }
    }
}
