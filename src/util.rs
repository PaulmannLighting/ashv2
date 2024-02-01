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
