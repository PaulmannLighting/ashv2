//! A three bit sequence number that wraps around.

use core::fmt::Display;

/// A three bit unsigned integer sequence number.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum Seq {
    #[default]
    Zero = 0,
    One = 1,
    Two = 2,
    Three = 3,
    Four = 4,
    Five = 5,
    Six = 6,
    Seven = 7,
}

impl Seq {
    /// Return the next number.
    #[must_use]
    pub const fn next(self) -> Self {
        match self {
            Self::Zero => Self::One,
            Self::One => Self::Two,
            Self::Two => Self::Three,
            Self::Three => Self::Four,
            Self::Four => Self::Five,
            Self::Five => Self::Six,
            Self::Six => Self::Seven,
            Self::Seven => Self::Zero,
        }
    }

    /// Increment the number.
    pub const fn increment(&mut self) {
        *self = self.next();
    }

    /// Convert the number into a `u8`.
    #[must_use]
    pub const fn as_u8(self) -> u8 {
        self as u8
    }
}

impl Display for Seq {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_u8())
    }
}

impl TryFrom<u8> for Seq {
    type Error = u8;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Zero),
            1 => Ok(Self::One),
            2 => Ok(Self::Two),
            3 => Ok(Self::Three),
            4 => Ok(Self::Four),
            5 => Ok(Self::Five),
            6 => Ok(Self::Six),
            7 => Ok(Self::Seven),
            _ => Err(value),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Seq;

    #[test]
    fn test_new() {
        for n in u8::MIN..=u8::MAX {
            let number = Seq::try_from(n % 8).unwrap();
            assert_eq!(number.as_u8(), n % 8);
        }
    }

    #[test]
    fn test_const_zero() {
        assert_eq!(Seq::Zero.as_u8(), 0);
    }

    #[test]
    fn test_as_u8() {
        for n in u8::MIN..=u8::MAX {
            let number = Seq::try_from(n % 8).unwrap();
            assert_eq!(number.as_u8(), n % 8);
        }
    }
}
