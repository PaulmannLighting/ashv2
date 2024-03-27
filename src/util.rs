use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

const MSG: &str = "Lock should never be poisoned.";

#[derive(Debug)]
pub struct NonPoisonedRwLock<T>(RwLock<T>);

impl<T> NonPoisonedRwLock<T> {
    #[must_use]
    pub const fn new(value: T) -> Self {
        Self(RwLock::new(value))
    }

    pub fn read(&self) -> RwLockReadGuard<'_, T> {
        self.0.read().expect(MSG)
    }

    pub fn write(&self) -> RwLockWriteGuard<'_, T> {
        self.0.write().expect(MSG)
    }
}

/// Return the respective next 3-bit number wrapping around on overflows.
pub const fn next_three_bit_number(number: u8) -> u8 {
    (number + 1) % 8
}

#[cfg(test)]
mod tests {
    use super::next_three_bit_number;

    #[test]
    fn test_next_three_bit_number() {
        for n in u8::MIN..u8::MAX {
            assert_eq!(
                next_three_bit_number(n),
                if n == u8::MAX { u8::MIN } else { n + 1 } % 8
            );
        }
    }
}
