pub mod types;
pub mod enc;
pub mod dec;

#[cfg(feature = "use_alloc")]
use alloc::{ vec::Vec, boxed::Box, string::String };


pub(crate) mod major {
    pub const UNSIGNED: u8 = 0;
    pub const NEGATIVE: u8 = 1;
    pub const BYTES:    u8 = 2;
    pub const STRING:   u8 = 3;
    pub const ARRAY:    u8 = 4;
    pub const MAP:      u8 = 5;
    pub const TAG:      u8 = 6;
    pub const SIMPLE:   u8 = 7;
}

pub(crate) mod marker {
    pub const START: u8 = 0x1f;
    pub const FALSE: u8 = 0xf4;
    pub const TRUE: u8  = 0xf5;
    pub const NULL: u8  = 0xf6;
    pub const UNDEFINED: u8 = 0xf7;
    pub const F16: u8   = 0xf9;
    pub const F32: u8   = 0xfa;
    pub const F64: u8   = 0xfb;
    pub const BREAK: u8   = 0xff;
}

#[cfg_attr(feature = "serde-value", derive(serde::Serialize, serde::Deserialize))]
#[cfg(feature = "use_alloc")]
#[derive(Debug, PartialEq)]
#[non_exhaustive]
pub enum Value {
    Null,
    Bool(bool),
    Integer(i128),
    Float(f64),
    Bytes(Vec<u8>),
    Text(String),
    Array(Vec<Value>),
    Map(Vec<(Value, Value)>),
    Tag(u64, Box<Value>)
}

#[cfg(feature = "use_alloc")]
impl enc::Encode for Value {
    fn encode<W: enc::Write>(&self, writer: &mut W) -> Result<(), enc::Error<W::Error>> {
        match self {
            Value::Null => types::Null.encode(writer),
            Value::Bool(v) => v.encode(writer),
            Value::Integer(v) => v.encode(writer),
            Value::Float(v) => v.encode(writer),
            Value::Bytes(v) => types::Bytes(v.as_slice()).encode(writer),
            Value::Text(v) => v.as_str().encode(writer),
            Value::Array(v) => v.as_slice().encode(writer),
            Value::Map(v) => types::Map(v.as_slice()).encode(writer),
            Value::Tag(tag, v) => types::Tag(*tag, &**v).encode(writer)
        }
    }
}

#[cfg(feature = "use_alloc")]
impl<'de> dec::Decode<'de> for Value {
    fn decode_with<R: dec::Read<'de>>(byte: u8, reader: &mut R) -> Result<Self, dec::Error<R::Error>> {
        use crate::util::ScopeGuard;

        if !reader.step_in() {
            return Err(dec::Error::RecursionLimit);
        }

        let mut reader = ScopeGuard(reader, |reader| reader.step_out());
        let reader = &mut *reader;

        match byte >> 5 {
            major::UNSIGNED => u64::decode_with(byte, reader)
                .map(|i| Value::Integer(i.into())),
            major::NEGATIVE => {
                let types::Negative(v) = <types::Negative<u64>>::decode_with(byte, reader)?;
                let v = i128::from(v);
                let v = v.checked_add(1)
                    .ok_or(dec::Error::Overflow { name: "Value::Integer" })?;
                Ok(Value::Integer(-v))
            },
            major::BYTES => <types::Bytes<Vec<u8>>>::decode_with(byte, reader)
                .map(|buf| Value::Bytes(buf.0)),
            major::STRING => String::decode_with(byte, reader)
                .map(Value::Text),
            major::ARRAY => <Vec<Value>>::decode_with(byte, reader)
                .map(Value::Array),
            major::MAP => <types::Map<Vec<(Value, Value)>>>::decode_with(byte, reader)
                .map(|map| Value::Map(map.0)),
            _ => match byte {
                marker::FALSE => Ok(Value::Bool(false)),
                marker::TRUE => Ok(Value::Bool(true)),
                marker::NULL | marker::UNDEFINED => Ok(Value::Null),
                #[cfg(feature = "half-f16")]
                marker::F16 => {
                    let v = half::f16::decode_with(byte, reader)?;
                    Ok(Value::Float(v.into()))
                },
                marker::F32 => f32::decode_with(byte, reader)
                    .map(|v| Value::Float(v.into())),
                marker::F64 => f64::decode_with(byte, reader)
                    .map(Value::Float),
                _ => Err(dec::Error::Unsupported { byte })
            }
        }
    }
}
