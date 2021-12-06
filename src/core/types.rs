pub struct Bytes<T>(pub T);

pub struct Tag<T>(pub u64, pub T);

pub struct Simple(pub u8);

pub struct Null;

pub struct Undefined;

pub struct F16(pub u16);
