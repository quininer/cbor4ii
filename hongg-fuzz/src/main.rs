use honggfuzz::fuzz;
use cbor4ii::core::Value;
use cbor4ii::serde::from_slice;

fn main() {
    loop {
        fuzz!(|data: &[u8]| {
            let _ = from_slice::<Value>(data);
        });
    }
}
