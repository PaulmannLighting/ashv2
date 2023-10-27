use itertools::Itertools;
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
    P: FnMut(&T) -> bool,
{
    fn extract(&mut self, predicate: P) -> Self {
        let indices = self.iter().positions(predicate).collect_vec();
        let mut result = Self::with_capacity(self.capacity());

        for index in indices {
            result.push(self.remove(index));
        }

        result
    }
}

impl<T, P> Extract<T, P> for VecDeque<T>
where
    P: FnMut(&T) -> bool,
{
    fn extract(&mut self, predicate: P) -> Self {
        let indices = self.iter().positions(predicate).collect_vec();
        let mut result = Self::with_capacity(self.capacity());

        for index in indices {
            if let Some(item) = self.remove(index) {
                result.push_front(item);
            }
        }

        result
    }
}
