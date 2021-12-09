#![no_main]

use std::convert::Infallible;
use libfuzzer_sys::fuzz_target;
use cbor4ii::core::Value;
use cbor4ii::core::dec::{ self, Decode };

struct SliceReader<'a> {
    buf: &'a [u8],
    depth: usize
}
impl SliceReader<'_> {
    fn new(buf: &[u8]) -> SliceReader<'_> {
        SliceReader { buf, depth: 0 }
    }
}

impl<'de> dec::Read<'de> for SliceReader<'de> {
    type Error = Infallible;

    fn fill<'b>(&'b mut self, want: usize) -> Result<dec::Reference<'de, 'b>, Self::Error> {
        let len = std::cmp::min(self.buf.len(), want);

        Ok(dec::Reference::Long(&self.buf[..len]))
    }

    fn advance(&mut self, n: usize) {
        debug_assert!(n <= self.buf.len());

        self.buf = &self.buf[n..];
    }

    fn step_in(&mut self) -> bool {
        let depth = self.depth + 1;
        if depth <= 256 {
            self.depth = depth;
            true
        } else {
            false
        }
    }

    fn step_out(&mut self) {
        self.depth -= 1;
    }
}

fuzz_target!(|data: &[u8]| {
    let mut reader = SliceReader::new(data);
    let _ = Value::decode(&mut reader);
});
