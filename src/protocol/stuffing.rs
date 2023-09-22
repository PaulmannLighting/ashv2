const FLAG: u8 = 0x7E;
const ESCAPE: u8 = 0x7D;
const X_ON: u8 = 0x11;
const X_OFF: u8 = 0x13;
const SUBSTITUTE: u8 = 0x18;
const CANCEL: u8 = 0x1A;
const RESERVED_BYTES: [u8; 6] = [FLAG, ESCAPE, X_ON, X_OFF, SUBSTITUTE, CANCEL];
const COMPLEMENT_BIT: u8 = 1 << 5;

pub struct Unstuffer<T>
where
    T: Iterator<Item = u8>,
{
    bytes: T,
}

/// Undo byte stuffing.
///
/// # Examples
/// ```
/// use ashv2::protocol::stuffing::Unstuffer;
///
/// let stuffed: [u8; 12] = [0x7D, 0x5E, 0x7D, 0x31, 0x7D, 0x33, 0x7D, 0x38, 0x7D, 0x3A, 0x7D, 0x5D];
/// let original = vec![0x7E, 0x11, 0x13, 0x18, 0x1A, 0x7D];
/// let unstuffer = Unstuffer::new(stuffed.into_iter());
/// let unstuffed: Vec<u8> = unstuffer.collect();
/// assert_eq!(unstuffed, original);
/// ```
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
