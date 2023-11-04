use crate::packet::MAX_FRAME_SIZE;
use crate::protocol::stuffing::{COMPLEMENT_BIT, RESERVED_BYTES};
use crate::protocol::{Unstuff, ESCAPE};
use std::io::{Error, ErrorKind, Read, Seek, SeekFrom, Write};
use std::ops::{Deref, DerefMut};

#[allow(clippy::module_name_repetitions)]
pub type FrameBuffer = Buffer<MAX_FRAME_SIZE>;

#[derive(Debug)]
pub struct Buffer<const SIZE: usize> {
    bytes: [u8; SIZE],
    pos: usize,
}

impl<const SIZE: usize> Buffer<SIZE> {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            bytes: [0; SIZE],
            pos: 0,
        }
    }

    pub fn extend<I>(&mut self, bytes: I) -> std::io::Result<()>
    where
        I: IntoIterator<Item = u8>,
    {
        for byte in bytes {
            self.write_all(&[byte])?;
        }

        Ok(())
    }

    #[must_use]
    pub const fn len(&self) -> usize {
        self.pos
    }

    #[must_use]
    pub const fn position(&self) -> usize {
        self.pos
    }
}

impl<const SIZE: usize> Deref for Buffer<SIZE> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.bytes[0..self.pos]
    }
}

impl<const SIZE: usize> DerefMut for Buffer<SIZE> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.bytes[0..self.pos]
    }
}

impl<const SIZE: usize> Read for Buffer<SIZE> {
    fn read(&mut self, mut buf: &mut [u8]) -> std::io::Result<usize> {
        buf.write(&self.bytes[self.pos..SIZE]).map(|len| {
            self.pos += len;
            len
        })
    }
}

impl<const SIZE: usize> Seek for Buffer<SIZE> {
    #[allow(clippy::cast_possible_truncation)]
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        let (base_pos, offset) = match pos {
            SeekFrom::Start(n) => {
                self.pos = n as usize;
                return Ok(n);
            }
            SeekFrom::End(n) => (SIZE as u64, n),
            SeekFrom::Current(n) => (self.pos as u64, n),
        };

        match base_pos.checked_add_signed(offset) {
            Some(n) => {
                self.pos = n as usize;
                Ok(self.pos as u64)
            }
            None => Err(Error::new(
                ErrorKind::InvalidInput,
                "invalid seek to a negative or overflowing position",
            )),
        }
    }
}

impl<const SIZE: usize> Write for Buffer<SIZE> {
    fn write(&mut self, mut buf: &[u8]) -> std::io::Result<usize> {
        let start = self.pos;
        buf.read(&mut self.bytes[start..SIZE]).map(|len| {
            self.pos += len;
            len
        })
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<const SIZE: usize> Unstuff for Buffer<SIZE> {
    fn unstuff(&mut self) {
        let mut pos = None;
        let mut found;

        loop {
            found = false;

            for (index, byte) in self.bytes[..self.pos]
                .iter()
                .enumerate()
                .skip(pos.map_or(0, |n| n + 1))
            {
                if *byte == ESCAPE {
                    pos = Some(index);
                    found = true;
                    break;
                }
            }

            if !found {
                break;
            }

            if let Some(index) = pos {
                self.bytes[index..].rotate_left(1);
                self.pos = self.pos.saturating_sub(1);

                if let Some(byte) = self.get_mut(index) {
                    if !RESERVED_BYTES.contains(byte) {
                        *byte ^= COMPLEMENT_BIT;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::buffer::FrameBuffer;
    use crate::protocol::Unstuff;
    use std::io::{Seek, Write};

    #[test]
    fn test_new() {
        let buffer = FrameBuffer::new();
        assert_eq!(buffer.len(), 0);
        assert_eq!(buffer.position(), 0);
    }

    #[test]
    fn test_read() {
        let mut buffer = FrameBuffer::new();
        buffer
            .write_all(&[1, 2, 3, 4])
            .expect("Could not write to buffer.");
        assert_eq!(buffer.len(), 4);
        assert_eq!(buffer.position(), 4);

        buffer
            .write_all(&[42, 1, 3, 3, 7])
            .expect("Could not write to buffer.");
        assert_eq!(buffer.len(), 9);
        assert_eq!(buffer.position(), 9);
    }

    #[test]
    fn test_deref() {
        let mut buffer = FrameBuffer::new();
        buffer
            .write_all(&[1, 2, 3, 4])
            .expect("Could not write to buffer.");
        assert_eq!(&*buffer, &[1, 2, 3, 4]);
        buffer
            .write_all(&[42, 1, 3, 3, 7])
            .expect("Could not write to buffer.");
        assert_eq!(&*buffer, &[1, 2, 3, 4, 42, 1, 3, 3, 7]);
    }

    #[test]
    fn test_rewind() {
        let mut buffer = FrameBuffer::new();
        buffer
            .write_all(&[1, 2, 3, 4])
            .expect("Could not write to buffer.");
        buffer.rewind().expect("Could not rewind buffer.");
        assert_eq!(&*buffer, &[]);
        buffer
            .write_all(&[42, 1, 3, 3, 7])
            .expect("Could not write to buffer.");
        assert_eq!(&*buffer, &[42, 1, 3, 3, 7]);
    }

    #[test]
    fn test_in_place_unstuff() {
        let mut stuffed = FrameBuffer::new();
        stuffed
            .write_all(&[
                0x7D, 0x5E, 0x7D, 0x31, 0x7D, 0x33, 0x7D, 0x38, 0x7D, 0x3A, 0x7D, 0x5D,
            ])
            .expect("Could not write to buffer.");
        assert_eq!(
            &*stuffed,
            &[0x7D, 0x5E, 0x7D, 0x31, 0x7D, 0x33, 0x7D, 0x38, 0x7D, 0x3A, 0x7D, 0x5D]
        );
        stuffed.unstuff();
        eprintln!("{:?}", stuffed.bytes);
        assert_eq!(&*stuffed, &[0x7E, 0x11, 0x13, 0x18, 0x1A, 0x7D]);
    }
}
