//! Randomization for masking bytes in protocol messages.

/// Mask bytes with pseudo-random numbers.
pub trait Mask {
    /// Masks bytes with pseudo-random numbers.
    fn mask(&mut self);
}

impl Mask for [u8] {
    fn mask(&mut self) {
        self.iter_mut()
            .zip(MaskGenerator::default())
            .for_each(|(byte, mask)| *byte ^= mask);
    }
}

/// Generate a stream of pseudo-random numbers.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MaskGenerator {
    random: u8,
    flag_bit: u8,
    mask: u8,
}

impl MaskGenerator {
    pub const DEFAULT_MASK: u8 = 0xB8;
    pub const DEFAULT_SEED: u8 = 0x42;
    pub const DEFAULT_FLAG_BIT: u8 = 0x01;

    #[must_use]
    pub const fn new(seed: u8, flag_bit: u8, mask: u8) -> Self {
        Self {
            random: seed,
            flag_bit,
            mask,
        }
    }
}

impl Default for MaskGenerator {
    fn default() -> Self {
        Self::new(
            Self::DEFAULT_SEED,
            Self::DEFAULT_FLAG_BIT,
            Self::DEFAULT_MASK,
        )
    }
}

/// Generates a pseudo-random number stream.
impl Iterator for MaskGenerator {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        let random = self.random;
        self.random >>= 1;

        if random & self.flag_bit != 0 {
            self.random ^= self.mask;
        }

        Some(random)
    }
}

#[cfg(test)]
mod tests {
    use super::{Mask, MaskGenerator};

    #[test]
    fn test_mask_generator() {
        let mask_generator = MaskGenerator::default();
        let first_five: Vec<_> = mask_generator.take(5).collect();
        assert_eq!(first_five, vec![0x42, 0x21, 0xA8, 0x54, 0x2A]);
    }

    #[test]
    fn test_mask_with_version_command() {
        let mut bytes = vec![0x00, 0x00, 0x00, 0x02];
        let original = bytes.clone();
        bytes.mask();
        assert_eq!(bytes, vec![0x42, 0x21, 0xA8, 0x56]);
        bytes.mask();
        assert_eq!(bytes, original);
    }

    #[test]
    fn test_mask_with_version_response() {
        let mut bytes = vec![0x00, 0x80, 0x00, 0x02, 0x02, 0x11, 0x30];
        let original = bytes.clone();
        bytes.mask();
        assert_eq!(bytes, vec![0x42, 0xA1, 0xA8, 0x56, 0x28, 0x04, 0x82]);
        bytes.mask();
        assert_eq!(bytes, original);
    }
}
