use crate::protocol::{CANCEL, ESCAPE, FLAG, SUBSTITUTE, X_OFF, X_ON};

const RESERVED_BYTES: [u8; 6] = [FLAG, ESCAPE, X_ON, X_OFF, SUBSTITUTE, CANCEL];
const COMPLEMENT_BIT: u8 = 1 << 5;

/// Trait to allow stuffing of byte iterators.
pub trait Stuff: Iterator<Item = u8> + Sized {
    /// Stuffs a byte stream.
    fn stuff(self) -> Stuffer<Self> {
        Stuffer::new(self.into_iter())
    }
}

impl<T> Stuff for T where T: Iterator<Item = u8> {}

#[derive(Clone, Debug, Eq, PartialEq)]
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Unstuffer<T>
where
    T: Iterator<Item = u8>,
{
    bytes: T,
}

pub trait Unstuff {
    fn unstuff(&mut self);
}

impl<const SIZE: usize> Unstuff for heapless::Vec<u8, SIZE> {
    fn unstuff(&mut self) {
        let mut last_escape: usize = 0;

        loop {
            if let Some(index) = self
                .iter()
                .skip(last_escape)
                .position(|&byte| byte == ESCAPE)
            {
                last_escape += index;

                if let Some(byte) = self.get_mut(last_escape + 1) {
                    if !RESERVED_BYTES.contains(byte) {
                        *byte ^= COMPLEMENT_BIT;
                    }
                }

                self.remove(last_escape);
                last_escape += 1; // Skip unescaped follow byte.
            } else {
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Stuffer, Unstuff};

    #[test]
    fn test_stuffer() {
        let original = vec![0x7E, 0x11, 0x13, 0x18, 0x1A, 0x7D];
        let target = vec![
            0x7D, 0x5E, 0x7D, 0x31, 0x7D, 0x33, 0x7D, 0x38, 0x7D, 0x3A, 0x7D, 0x5D,
        ];
        let stuffer = Stuffer::new(original.into_iter());
        let stuffed_bytes: Vec<u8> = stuffer.collect();
        assert_eq!(stuffed_bytes, target);
    }

    #[test]
    fn test_in_place_unstuff() {
        let stuffed: [u8; 12] = [
            0x7D, 0x5E, 0x7D, 0x31, 0x7D, 0x33, 0x7D, 0x38, 0x7D, 0x3A, 0x7D, 0x5D,
        ];
        let mut buffer: heapless::Vec<u8, 12> = heapless::Vec::new();
        buffer.extend(stuffed);
        buffer.unstuff();
        assert_eq!(&buffer, &[0x7E, 0x11, 0x13, 0x18, 0x1A, 0x7D]);
    }

    #[test]
    fn test_unstuff_unchanged() {
        let payload:heapless::Vec<_,70>  = b"\xd7\x90\xd7\xa0\xd7\x99 \xd7\x96\xd7\x95\xd7\x9b\xd7\xa8 \xd7\x91\xd7\x9c\xd7\x99\xd7\x9c\xd7\x95\xd7\xaa \xd7\xa9\xd7\x9c \xd7\x99\xd7\xa8\xd7\x97 \xd7\x9e\xd7\x9c\xd7\x90 \xd7\x94\xd7\x99\xd7\x99\xd7\xaa \xd7\x91\xd7\x90\xd7\x94 \xd7\x90\xd7\x9c\xd7\x99".iter().copied().collect();
        let mut clone = payload.clone();
        clone.unstuff();
        assert_eq!(payload, clone);
    }
}
