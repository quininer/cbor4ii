pub enum Token {
    Null,
    Unsigned,
    Negative,
    Float,
    Bytes,
    String,
    Array,
    Map,
    Simple,
    End
}

pub enum Type {
    Null,
    Undefined,
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    N8(u8),
    N16(u16),
    N32(u32),
    N64(u64),
    F16(u16),
    F32(f32),
    F64(f64),
    Bytes(usize),
    String(usize),
    Array(usize),
    Map(usize),
    Simple(u8),
    Tag(u8)
}

impl Token {
    fn parse(x: u8) -> Token {
        todo!()
    }

    fn want(&self) -> usize {
        todo!()
    }

    fn read(&self, input: &[u8]) -> Type {
        todo!()
    }
}
