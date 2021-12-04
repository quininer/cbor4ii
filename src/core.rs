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
