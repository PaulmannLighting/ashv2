const DEFAULT_MASK: u8 = 0xB8;
const DEFAULT_SEED: u8 = 0x42;

/// Masks a byte stream with pseudo-random numbers.
pub fn mask(bytes: impl Iterator<Item = u8>) -> impl Iterator<Item = u8> {
    bytes
        .zip(MaskGenerator::default())
        .map(|(byte, mask)| byte ^ mask)
}

struct MaskGenerator {
    random: u8,
    mask: u8,
}

impl MaskGenerator {
    #[must_use]
    pub const fn new(seed: u8, mask: u8) -> Self {
        Self { random: seed, mask }
    }
}

impl Default for MaskGenerator {
    fn default() -> Self {
        Self::new(DEFAULT_SEED, DEFAULT_MASK)
    }
}

/// Generates a pseudo-random number stream.
impl Iterator for MaskGenerator {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        let random = self.random;

        self.random = if self.random & 0x01 == 0 {
            self.random >> 1
        } else {
            (self.random >> 1) ^ self.mask
        };

        Some(random)
    }
}

#[cfg(test)]
mod tests {
    use super::{mask, MaskGenerator};

    #[test]
    fn test_mask_generator() {
        let mask_generator = MaskGenerator::default();
        let first_five: Vec<_> = mask_generator.take(5).collect();
        assert_eq!(first_five, vec![0x42, 0x21, 0xA8, 0x54, 0x2A]);
    }

    #[test]
    fn test_mask_with_version_command() {
        let bytes = vec![0x00, 0x00, 0x00, 0x02];
        let masked: Vec<_> = mask(bytes.clone().into_iter()).collect();
        assert_eq!(masked, vec![0x42, 0x21, 0xA8, 0x56]);
        let unmasked: Vec<_> = mask(masked.into_iter()).collect();
        assert_eq!(unmasked, bytes);
    }

    #[test]
    fn test_mask_with_version_response() {
        let bytes = vec![0x00, 0x80, 0x00, 0x02, 0x02, 0x11, 0x30];
        let masked: Vec<_> = mask(bytes.clone().into_iter()).collect();
        assert_eq!(masked, vec![0x42, 0xA1, 0xA8, 0x56, 0x28, 0x04, 0x82]);
        let unmasked: Vec<_> = mask(masked.into_iter()).collect();
        assert_eq!(unmasked, bytes);
    }
}
