#![no_main]

use libfuzzer_sys::fuzz_target;
use cbor4ii::core::{ Value, RawValue };
use cbor4ii::core::dec::Decode;
use cbor4ii::core::utils::SliceReader;

fuzz_target!(|data: &[u8]| {
    let mut reader = SliceReader::new(data);
    let _ = Value::decode(&mut reader);
    let mut reader = SliceReader::new(data);
    let _ = RawValue::decode(&mut reader);
});
