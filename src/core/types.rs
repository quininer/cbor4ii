pub mod big {
    use alloc::vec::Vec;

    pub struct Num(pub Vec<u8>);
    pub struct Float(pub Vec<u8>);
}

pub struct Negative<T>(pub T);

pub struct Bytes<T>(pub T);

pub struct BadStr<T>(pub T);

pub struct Decimal(pub u64, pub u64);

pub struct Tag;

pub struct Simple(pub u8);

pub struct Undefined;
