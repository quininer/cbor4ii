pub mod types;
pub mod enc;
mod dec;

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

#[cfg(feature = "use_alloc")]
#[non_exhaustive]
pub enum Value {
    Null,
    Integer(i128),
    Float(f64),
    Bytes(Vec<u8>),
    Text(String),
    List(Vec<Value>),
    Map(Vec<(Value, Value)>),
    Tag(u64, Box<Value>)
}

#[cfg(feature = "use_alloc")]
impl enc::Encode for Value {
    fn encode<W: enc::Write>(&self, writer: &mut W) -> Result<(), enc::Error<W::Error>> {
        match self {
            Value::Null => types::Null.encode(writer),
            Value::Integer(v) => v.encode(writer),
            Value::Float(v) => v.encode(writer),
            Value::Bytes(v) => types::Bytes(v.as_slice()).encode(writer),
            Value::Text(v) => v.as_str().encode(writer),
            Value::List(v) => v.as_slice().encode(writer),
            Value::Map(v) => types::Map(v.as_slice()).encode(writer),
            Value::Tag(tag, v) => types::Tag(*tag, &**v).encode(writer)
        }
    }
}
