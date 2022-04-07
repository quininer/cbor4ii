#![no_main]

use std::convert::Infallible;
use libfuzzer_sys::fuzz_target;
use cbor4ii::core::Value;
use cbor4ii::core::dec::{ self, Decode };
use cbor4ii::core::utils::SliceReader;

fuzz_target!(|data: &[u8]| {
    let mut reader = SliceReader::new(data);
    let _ = Value::decode(&mut reader);
});
