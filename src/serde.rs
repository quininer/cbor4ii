pub mod ser;

use std::io;
use serde::Serialize;

struct IoWrite<W>(W);

impl<W: io::Write> crate::core::enc::Write for IoWrite<W> {
    type Error = io::Error;

    fn push(&mut self, input: &[u8]) -> Result<(), Self::Error> {
        self.0.write_all(input)
    }
}

pub fn to_writer<T, W>(value: &T, writer: &mut W)
    -> Result<(), crate::core::enc::Error<io::Error>>
where
    T: Serialize,
    W: io::Write
{
    let writer = IoWrite(writer);
    let mut writer = ser::Serializer::new(writer);
    value.serialize(&mut writer)
}


#[test]
fn test_serde_to_writer() {
    use std::fmt::Debug;
    use serde::de::DeserializeOwned;

    let value = vec![
        Some(0x99u32),
        None,
        Some(0x33u32)
    ];

    let mut output = Vec::new();
    to_writer(&value, &mut output).unwrap();

    #[track_caller]
    fn assert_value<T: Eq + Debug + DeserializeOwned>(bytes: &[u8], value: T) {
        let value2: T = serde_cbor::from_slice(bytes).unwrap();
        assert_eq!(value, value2);
    }

    assert_value(&output, value);
}
