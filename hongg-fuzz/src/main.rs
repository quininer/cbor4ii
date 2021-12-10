use std::convert::Infallible;
use honggfuzz::fuzz;
use cbor4ii::core::Value;
use cbor4ii::core::dec::{ self, Decode };
use cbor4ii::DecodeError;
use cbor4ii::serde::de;


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
