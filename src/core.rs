pub mod types;
pub mod enc;
mod dec;

use alloc::{ vec::Vec, boxed::Box, string::String };


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

impl enc::Encode for Value {
    fn encode<W: enc::Write>(&self, writer: &mut W) -> Result<(), enc::Error<W::Error>> {
        match self {
            Value::Null => types::Null.encode(writer),
            Value::Integer(v) => v.encode(writer),
            Value::Float(v) => v.encode(writer),
            Value::Bytes(v) => types::Bytes(v.as_slice()).encode(writer),
            Value::Text(v) => v.as_str().encode(writer),
            Value::List(v) => v.as_slice().encode(writer),
            Value::Map(v) => enc::Map(v.as_slice()).encode(writer),
            Value::Tag(tag, v) => types::Tag(*tag, &**v).encode(writer)
        }
    }
}
