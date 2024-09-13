use crate::protocol::{CANCEL, ESCAPE, FLAG, SUBSTITUTE, X_OFF, X_ON};

const RESERVED_BYTES: [u8; 6] = [FLAG, ESCAPE, X_ON, X_OFF, SUBSTITUTE, CANCEL];
const COMPLEMENT_BIT: u8 = 1 << 5;

/// Trait to allow stuffing of byte iterators.
pub trait Stuff {
    /// Stuffs a byte stream.
    fn stuff(&mut self);
}

impl<const SIZE: usize> Stuff for heapless::Vec<u8, SIZE> {
    fn stuff(&mut self) {
        let mut index: usize = 0;

        while index < self.len() {
            let byte = &mut self[index];

            if RESERVED_BYTES.contains(byte) {
                *byte ^= COMPLEMENT_BIT;
                self.insert(index, ESCAPE)
                    .expect("could not insert escape byte");
                index += 2;
            } else {
                index += 1;
            }
        }
    }
}

pub trait Unstuff {
    fn unstuff(&mut self);
}

impl<const SIZE: usize> Unstuff for heapless::Vec<u8, SIZE> {
    fn unstuff(&mut self) {
        let mut offset: usize = 0;

        while let Some(index) = self.iter().skip(offset).position(|&byte| byte == ESCAPE) {
            offset += index;

            let Some(byte) = self.get_mut(offset + 1) else {
                break;
            };

            if !RESERVED_BYTES.contains(byte) {
                *byte ^= COMPLEMENT_BIT;
            }

            self.remove(offset);
            offset += 1; // Skip unescaped follow byte.
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Stuff, Unstuff};

    #[test]
    fn test_stuffing() {
        let mut unstuffed: heapless::Vec<u8, 12> =
            [0x7E, 0x11, 0x13, 0x18, 0x1A, 0x7D].into_iter().collect();
        let stuffed = [
            0x7D, 0x5E, 0x7D, 0x31, 0x7D, 0x33, 0x7D, 0x38, 0x7D, 0x3A, 0x7D, 0x5D,
        ];
        unstuffed.stuff();
        assert_eq!(unstuffed.as_slice(), stuffed.as_slice());
    }

    #[test]
    fn test_unstuffing() {
        let mut stuffed: heapless::Vec<u8, 12> = [
            0x7D, 0x5E, 0x7D, 0x31, 0x7D, 0x33, 0x7D, 0x38, 0x7D, 0x3A, 0x7D, 0x5D,
        ]
        .into_iter()
        .collect();
        let unstuffed = [0x7E, 0x11, 0x13, 0x18, 0x1A, 0x7D];
        stuffed.unstuff();
        assert_eq!(stuffed.as_slice(), unstuffed.as_slice());
    }

    #[test]
    fn test_unstuffing_unchanged() {
        let payload: heapless::Vec<_, 70> = [
            0xd7, 0x90, 0xd7, 0xa0, 0xd7, 0x99, 0x20, 0xd7, 0x96, 0xd7, 0x95, 0xd7, 0x9b, 0xd7,
            0xa8, 0x20, 0xd7, 0x91, 0xd7, 0x9c, 0xd7, 0x99, 0xd7, 0x9c, 0xd7, 0x95, 0xd7, 0xaa,
            0x20, 0xd7, 0xa9, 0xd7, 0x9c, 0x20, 0xd7, 0x99, 0xd7, 0xa8, 0xd7, 0x97, 0x20, 0xd7,
            0x9e, 0xd7, 0x9c, 0xd7, 0x90, 0x20, 0xd7, 0x94, 0xd7, 0x99, 0xd7, 0x99, 0xd7, 0xaa,
            0x20, 0xd7, 0x91, 0xd7, 0x90, 0xd7, 0x94, 0x20, 0xd7, 0x90, 0xd7, 0x9c, 0xd7, 0x99,
        ]
        .into_iter()
        .collect();
        let mut clone = payload.clone();
        clone.unstuff();
        assert_eq!(clone.as_slice(), payload.as_slice());
    }
}
