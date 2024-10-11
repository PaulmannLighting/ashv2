use std::fmt::{LowerHex, UpperHex};

/// A wrapper around a slice of bytes to format it with hexadecimal bytes.
pub struct HexSlice<'a>(&'a [u8]);

impl<'a> HexSlice<'a> {
    /// Creates a new `HexSlice` from a slice of bytes.
    pub const fn new(slice: &'a [u8]) -> Self {
        Self(slice)
    }

    /// Returns the maximum index of the slice.
    const fn max_index(&self) -> usize {
        self.0.len().saturating_sub(1)
    }
}

impl UpperHex for HexSlice<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[")?;

        for (index, byte) in self.0.iter().enumerate() {
            UpperHex::fmt(byte, f)?;

            if index != self.max_index() {
                write!(f, ", ")?;
            }
        }

        write!(f, "]")
    }
}

impl LowerHex for HexSlice<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[")?;

        for (index, byte) in self.0.iter().enumerate() {
            UpperHex::fmt(byte, f)?;

            if index != self.max_index() {
                write!(f, ", ")?;
            }
        }

        write!(f, "]")
    }
}

#[cfg(test)]
mod tests {
    use super::HexSlice;

    #[test]
    fn test_upper_hex() {
        let slice = HexSlice::new(&[0x01, 0x02, 0x03]);
        assert_eq!(format!("{slice:#04X}"), "[0x01, 0x02, 0x03]");
    }

    #[test]
    fn test_lower_hex() {
        let slice = HexSlice::new(&[0x01, 0x02, 0x03]);
        assert_eq!(format!("{slice:#04x}"), "[0x01, 0x02, 0x03]");
    }
}