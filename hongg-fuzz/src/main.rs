use std::convert::Infallible;
use honggfuzz::fuzz;
use cbor4ii::core::Value;
use cbor4ii::core::dec::Decode;
use cbor4ii::core::utils::SliceReader;
use cbor4ii::DecodeError;
use cbor4ii::serde::Deserializer;


pub fn from_slice<'a, T>(buf: &'a [u8]) -> Result<T, DecodeError<Infallible>>
    where
        T: serde::Deserialize<'a>,
    {
        let reader = SliceReader::new(buf);
        let mut deserializer = Deserializer::new(reader);
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
