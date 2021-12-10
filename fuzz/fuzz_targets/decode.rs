#![no_main]

use std::convert::Infallible;
use libfuzzer_sys::fuzz_target;
use cbor4ii::core::Value;
use cbor4ii::core::dec::{ self, Decode };

struct SliceReader<'a> {
    buf: &'a [u8],
    limit: usize
}

impl<'de> dec::Read<'de> for SliceReader<'de> {
    type Error = Infallible;

    #[inline]
    fn fill<'b>(&'b mut self, want: usize) -> Result<dec::Reference<'de, 'b>, Self::Error> {
        let len = core::cmp::min(self.buf.len(), want);
        Ok(dec::Reference::Long(&self.buf[..len]))
    }

    #[inline]
    fn advance(&mut self, n: usize) {
        let len = core::cmp::min(self.buf.len(), n);
        self.buf = &self.buf[len..];
    }

    #[inline]
    fn step_in(&mut self) -> bool {
        if let Some(limit) = self.limit.checked_sub(1) {
            self.limit = limit;
            true
        } else {
            false
        }
    }

    #[inline]
    fn step_out(&mut self) {
        self.limit += 1;
    }
}

fuzz_target!(|data: &[u8]| {
    let mut reader = SliceReader::new(data);
    let _ = Value::decode(&mut reader);
});
