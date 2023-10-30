use crate::packet::Data;
use crate::protocol::{Mask, Stuffing};
use itertools::Itertools;
use log::debug;
use std::io;
use std::io::Read;
use std::sync::Arc;

const INITIAL_BUFFER_CAPACITY: usize = 220;

#[derive(Debug, Eq, PartialEq)]
pub struct Input {
    data: Vec<Data>,
    buffer: Vec<u8>,
    byte: [u8; 1],
}

impl Input {
    pub fn buffer(&self) -> &[u8] {
        self.buffer.as_slice()
    }

    pub fn buffer_mut(&mut self) -> &mut Vec<u8> {
        &mut self.buffer
    }

    pub fn bytes(&self) -> Arc<[u8]> {
        self.data
            .iter()
            .dedup_by(|lhs, rhs| lhs.frame_num() == rhs.frame_num())
            .flat_map(|data| data.payload().iter().copied().mask())
            .collect()
    }

    pub fn clear(&mut self) {
        debug!("Clearing input buffers.");
        self.data.clear();
        self.buffer.clear();
        self.byte = [0];
    }

    pub fn frame_bytes(&self) -> Vec<u8> {
        self.buffer.iter().copied().unstuff().collect()
    }

    pub fn push_data(&mut self, data: Data) {
        self.data.push(data);
    }

    pub fn read_byte<R>(&mut self, src: &mut R) -> io::Result<u8>
    where
        R: Read,
    {
        src.read_exact(&mut self.byte)?;
        Ok(self.byte[0])
    }
}

impl Default for Input {
    fn default() -> Self {
        Self {
            data: Vec::new(),
            buffer: Vec::with_capacity(INITIAL_BUFFER_CAPACITY),
            byte: [0],
        }
    }
}
