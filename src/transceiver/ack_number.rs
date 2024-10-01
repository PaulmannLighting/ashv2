use std::num::NonZero;

const MOD: u8 = 8;

/// An optional three bit number.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct AckNumber(Option<NonZero<u8>>);

impl AckNumber {
    #[must_use]
    pub const fn new() -> Self {
        Self(None)
    }

    pub fn increment(&mut self) {
        if let Some(n) = self.0 {
            self.0 = NonZero::new(n.get().wrapping_add(1) % MOD);
        } else {
            self.0 = NonZero::new(1);
        }
    }
}

impl From<u8> for AckNumber {
    fn from(value: u8) -> Self {
        Self(NonZero::new(value << 1))
    }
}

impl From<AckNumber> for Option<u8> {
    fn from(value: AckNumber) -> Self {
        value.0.map(|n| n.get() >> 1)
    }
}
