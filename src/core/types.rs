pub mod big {
    use alloc::vec::Vec;

    pub struct Num(pub Vec<u8>);
    pub struct Float(pub Vec<u8>);
}

pub struct Decimal(pub u64, pub u64);

pub struct Tag;

pub struct Simple(pub u8);

pub struct Undefined;
