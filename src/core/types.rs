pub struct Negative<T>(pub T);

pub struct Bytes<T>(pub T);

pub struct BadStr<T>(pub T);

pub struct Tag<T>(pub u8, pub T);

pub struct Simple(pub u8);

pub struct Undefined;

pub struct F16(pub u16);
