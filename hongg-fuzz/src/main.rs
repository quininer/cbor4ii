use std::convert::Infallible;
use honggfuzz::fuzz;
use cbor4ii::core::Value;
use cbor4ii::core::dec::{ self, Decode };
use cbor4ii::DecodeError;
use cbor4ii::serde::de;


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

pub fn from_slice<'a, T>(buf: &'a [u8]) -> Result<T, DecodeError<Infallible>>
    where
        T: serde::Deserialize<'a>,
    {
        let reader = SliceReader::new(buf);
        let mut deserializer = de::Deserializer::new(reader);
        serde::Deserialize::deserialize(&mut deserializer)
    }

fn main() {
    loop {
        fuzz!(|data: &[u8]| {
            // decode
            {
                let mut reader = SliceReader::new(data);
                let _ = Value::decode(&mut reader);
            }

            // serde
            {
                let _ = from_slice::<Value>(data);
            }
        });
    }
}
