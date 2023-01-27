#![no_main]

use libfuzzer_sys::fuzz_target;
use serde::Deserialize;
use cbor4ii::core::Value;
use cbor4ii::serde::from_slice;


#[derive(Deserialize)]
struct Bar {
    _a: u32
}

fuzz_target!(|data: &[u8]| {
    let _ = from_slice::<Bar>(data);
    let _ = from_slice::<Value>(data);
});
