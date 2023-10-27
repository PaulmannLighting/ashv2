mod input;
mod output;

use input::Input;
use output::Output;

const INITIAL_BUFFER_CAPACITY: usize = 220;

#[derive(Debug, Default, Eq, PartialEq)]
pub struct Buffers {
    pub input: Input,
    pub output: Output,
}

impl Buffers {
    pub fn clear(&mut self) {
        self.input.clear();
        self.output.clear();
    }
}
