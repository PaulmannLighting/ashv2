use crate::protocol::{CANCEL, ESCAPE, FLAG, SUBSTITUTE, X_OFF, X_ON};

pub const RESERVED_BYTES: [u8; 6] = [FLAG, ESCAPE, X_ON, X_OFF, SUBSTITUTE, CANCEL];
pub const COMPLEMENT_BIT: u8 = 1 << 5;

/// Trait to allow stuffing and unstuffing of byte iterators.
pub trait Stuffing: Iterator<Item = u8> + Sized {
    /// Stuffs a byte stream.
    fn stuff(self) -> Stuffer<Self> {
        Stuffer::new(self)
    }

    /// Unstuffs a byte stream.
    fn unstuff(self) -> Unstuffer<Self> {
        Unstuffer::new(self)
    }
}

impl<T> Stuffing for T where T: Iterator<Item = u8> {}

pub struct Stuffer<T>
where
    T: Iterator<Item = u8>,
{
    bytes: T,
    next: Option<u8>,
}

/// Stuff bytes.
impl<T> Stuffer<T>
where
    T: Iterator<Item = u8>,
{
    pub const fn new(bytes: T) -> Self {
        Self { bytes, next: None }
    }
}

impl<T> Iterator for Stuffer<T>
where
    T: Iterator<Item = u8>,
{
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(next) = self.next.take() {
            Some(next)
        } else if let Some(byte) = self.bytes.next() {
            if RESERVED_BYTES.contains(&byte) {
                self.next = Some(byte ^ COMPLEMENT_BIT);
                Some(ESCAPE)
            } else {
                Some(byte)
            }
        } else {
            None
        }
    }
}

pub struct Unstuffer<T>
where
    T: Iterator<Item = u8>,
{
    bytes: T,
}

/// Undo byte stuffing.
impl<T> Unstuffer<T>
where
    T: Iterator<Item = u8>,
{
    pub const fn new(bytes: T) -> Self {
        Self { bytes }
    }
}

impl<T> Iterator for Unstuffer<T>
where
    T: Iterator<Item = u8>,
{
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        self.bytes.next().and_then(|byte| {
            if byte == ESCAPE {
                self.bytes.next().map(|byte| {
                    if RESERVED_BYTES.contains(&byte) {
                        byte
                    } else {
                        byte ^ COMPLEMENT_BIT
                    }
                })
            } else {
                Some(byte)
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{Stuffer, Stuffing, Unstuffer};

    #[test]
    fn test_stuffing_trait() {
        let original = vec![0x7E, 0x11, 0x13, 0x18, 0x1A, 0x7D];
        let stuffed_and_unstuffed: Vec<u8> =
            original.clone().into_iter().stuff().unstuff().collect();
        assert_eq!(stuffed_and_unstuffed, original);
    }

    #[test]
    fn test_stuffer() {
        let original = vec![0x7E, 0x11, 0x13, 0x18, 0x1A, 0x7D];
        let target = vec![
            0x7D, 0x5E, 0x7D, 0x31, 0x7D, 0x33, 0x7D, 0x38, 0x7D, 0x3A, 0x7D, 0x5D,
        ];
        let stuffer = Stuffer::new(original.into_iter());
        let stuffed: Vec<u8> = stuffer.collect();
        assert_eq!(stuffed, target);
    }

    #[test]
    fn test_unstuffer() {
        let stuffed: [u8; 12] = [
            0x7D, 0x5E, 0x7D, 0x31, 0x7D, 0x33, 0x7D, 0x38, 0x7D, 0x3A, 0x7D, 0x5D,
        ];
        let original = vec![0x7E, 0x11, 0x13, 0x18, 0x1A, 0x7D];
        let unstuffer = Unstuffer::new(stuffed.into_iter());
        let unstuffed: Vec<u8> = unstuffer.collect();
        assert_eq!(unstuffed, original);
    }
}