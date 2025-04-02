use std::fmt::{Formatter, LowerHex, Result, UpperHex};

/// A wrapper around a slice of bytes to format it with hexadecimal bytes.
pub struct HexSlice<'a>(&'a [u8]);

impl<'a> HexSlice<'a> {
    /// Creates a new `HexSlice` from a slice of bytes.
    #[must_use]
    pub const fn new(slice: &'a [u8]) -> Self {
        Self(slice)
    }

    /// Returns the maximum index of the slice.
    const fn max_index(&self) -> usize {
        self.0.len().saturating_sub(1)
    }

    /// Formats the slice with a custom byte formatter.
    fn format<F>(&self, f: &mut Formatter<'_>, byte_formatter: F) -> Result
    where
        F: Fn(&u8, &mut Formatter<'_>) -> Result,
    {
        write!(f, "[")?;

        for (index, byte) in self.0.iter().enumerate() {
            byte_formatter(byte, f)?;

            if index != self.max_index() {
                write!(f, ", ")?;
            }
        }

        write!(f, "]")
    }
}

impl<'a> From<&'a [u8]> for HexSlice<'a> {
    fn from(slice: &'a [u8]) -> Self {
        Self::new(slice)
    }
}

impl UpperHex for HexSlice<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        self.format(f, UpperHex::fmt)
    }
}

impl LowerHex for HexSlice<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        self.format(f, LowerHex::fmt)
    }
}

#[cfg(test)]
mod tests {
    use super::HexSlice;

    #[test]
    fn test_from() {
        let slice: &[u8] = &[0x01, 0xAB, 0x03];
        let hex_slice: HexSlice<'_> = slice.into();
        assert_eq!(hex_slice.0, slice);
    }

    #[test]
    fn test_upper_hex() {
        let slice = HexSlice::new(&[0x01, 0xAB, 0x03]);
        assert_eq!(format!("{slice:#04X}"), "[0x01, 0xAB, 0x03]");
    }

    #[test]
    fn test_lower_hex() {
        let slice = HexSlice::new(&[0x01, 0xAB, 0x03]);
        assert_eq!(format!("{slice:#04x}"), "[0x01, 0xab, 0x03]");
    }

    #[test]
    fn test_empty_slice() {
        let slice = HexSlice::new(&[]);
        assert_eq!(format!("{slice:#04x}"), "[]");
    }

    #[test]
    fn test_empty_slice_len_1_upper() {
        let slice = HexSlice::new(&[0x1A]);
        assert_eq!(format!("{slice:#04X}"), "[0x1A]");
    }

    #[test]
    fn test_empty_slice_len_1_lower() {
        let slice = HexSlice::new(&[0x1A]);
        assert_eq!(format!("{slice:#04x}"), "[0x1a]");
    }
}
