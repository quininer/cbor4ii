#![no_main]

use libfuzzer_sys::fuzz_target;
use cbor4ii::core::Value;
use cbor4ii::serde::from_slice;


fuzz_target!(|data: &[u8]| {
    let _ = from_slice::<Value>(data);
});
