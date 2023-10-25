use std::iter::{Map, Zip};

const DEFAULT_MASK: u8 = 0xB8;
const DEFAULT_SEED: u8 = 0x42;
const DEFAULT_FLAG_BIT: u8 = 0x01;

type MaskIterator<T> = Map<Zip<T, MaskGenerator>, fn((u8, u8)) -> u8>;

pub trait Mask: IntoIterator<Item = u8> + Sized {
    /// Masks a byte stream with pseudo-random numbers.
    fn mask(self) -> MaskIterator<<Self as IntoIterator>::IntoIter> {
        self.into_iter()
            .zip(MaskGenerator::default())
            .map(|(byte, mask)| byte ^ mask)
    }
}

impl<T> Mask for T where T: IntoIterator<Item = u8> {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MaskGenerator {
    random: u8,
    flag_bit: u8,
    mask: u8,
}

impl MaskGenerator {
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
        Self::new(DEFAULT_SEED, DEFAULT_FLAG_BIT, DEFAULT_MASK)
    }
}

/// Generates a pseudo-random number stream.
impl Iterator for MaskGenerator {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        let random = self.random;

        self.random = if self.random & self.flag_bit == 0 {
            self.random >> 1
        } else {
            (self.random >> 1) ^ self.mask
        };

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
        let bytes = vec![0x00, 0x00, 0x00, 0x02];
        let masked: Vec<_> = bytes.clone().into_iter().mask().collect();
        assert_eq!(masked, vec![0x42, 0x21, 0xA8, 0x56]);
        let unmasked: Vec<_> = masked.into_iter().mask().collect();
        assert_eq!(unmasked, bytes);
    }

    #[test]
    fn test_mask_with_version_response() {
        let bytes = vec![0x00, 0x80, 0x00, 0x02, 0x02, 0x11, 0x30];
        let masked: Vec<_> = bytes.clone().into_iter().mask().collect();
        assert_eq!(masked, vec![0x42, 0xA1, 0xA8, 0x56, 0x28, 0x04, 0x82]);
        let unmasked: Vec<_> = masked.into_iter().mask().collect();
        assert_eq!(unmasked, bytes);
    }
}
