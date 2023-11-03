use std::collections::VecDeque;

pub trait Extract<T, P>
where
    Self: IntoIterator<Item = T>,
    P: FnMut(&T) -> bool,
{
    /// Removes the elements for which `predicate` returns `true` from the collection
    /// and returns those in kept order as a new collection of the same type.
    fn extract(&mut self, predicate: P) -> Self;
}

impl<T, P> Extract<T, P> for Vec<T>
where
    P: FnMut(&T) -> bool + Clone,
{
    fn extract(&mut self, predicate: P) -> Self {
        let mut result = Self::with_capacity(self.len());

        while let Some(index) = self.iter().position(predicate.clone()) {
            result.push(self.remove(index));
        }

        result
    }
}

impl<T, P> Extract<T, P> for VecDeque<T>
where
    P: FnMut(&T) -> bool + Clone,
{
    fn extract(&mut self, predicate: P) -> Self {
        let mut result = Self::with_capacity(self.len());

        while let Some(index) = self.iter().position(predicate.clone()) {
            if let Some(item) = self.remove(index) {
                result.push_back(item);
            }
        }

        result
    }
}

pub const fn next_three_bit_number(number: u8) -> u8 {
    (number + 1) % 8
}
