#![cfg(feature = "use_alloc")]

use std::convert::Infallible;
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
        if depth <= 124 {
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

#[test]
fn test_decode_value() {
    macro_rules! test {
        ( @ $bytes:expr ) => {
            let mut reader = SliceReader::new($bytes);
            let _ = Value::decode(&mut reader);
        };
        ( $( $bytes:expr );* $( ; )? ) => {
            $(
                test!(@ $bytes );
            )*
        }
    }

    test!{
        &[0x8a];
        &[0x7a, 0x86];
        include_bytes!("fuzz_data1.bin");
    }
}
