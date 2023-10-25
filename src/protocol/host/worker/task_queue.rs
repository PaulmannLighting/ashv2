use std::collections::VecDeque;

#[derive(Debug)]
pub struct TaskQueue<T> {
    queue: VecDeque<(usize, T)>,
    next_id: usize,
}

impl<T> TaskQueue<T> {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            queue: VecDeque::new(),
            next_id: 0,
        }
    }

    pub fn push(&mut self, item: T) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        self.queue.push_front((id, item));
        id
    }

    #[must_use]
    pub fn pop(&mut self) -> Option<(usize, T)> {
        self.queue.pop_back()
    }
}
