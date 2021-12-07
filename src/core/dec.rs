use crate::core::{ major, types };
pub use crate::error::DecodeError as Error;

#[cfg(feature = "use_alloc")]
use alloc::{ vec::Vec, string::String };

pub trait Read<'a> {
    #[cfg(feature = "use_std")]
    type Error: std::error::Error + 'static;

    #[cfg(not(feature = "use_std"))]
    type Error: core::fmt::Display + core::fmt::Debug;

    fn fill<'b>(&'b mut self, want: usize) -> Result<Reference<'a, 'b>, Self::Error>;
    fn advance(&mut self, n: usize);
}

pub enum Reference<'a, 'b> {
    Long(&'a [u8]),
    Short(&'b [u8])
}

pub trait Decode<'a>: Sized {
    fn decode<R: Read<'a>>(reader: &mut R) -> Result<Self, Error<R::Error>>;
}

impl Reference<'_, '_> {
    #[inline]
    const fn as_ref(&self) -> &[u8] {
        match self {
            Reference::Long(buf) => buf,
            Reference::Short(buf) => buf
        }
    }
}

#[inline]
fn pull_one<'a, R: Read<'a>>(reader: &mut R) -> Result<u8, Error<R::Error>> {
    let b = reader.fill(1)?
        .as_ref()
        .get(0)
        .copied()
        .ok_or(Error::Eof)?;
    reader.advance(1);
    Ok(b)
}

#[inline]
fn pull_exact<'a, R: Read<'a>>(reader: &mut R, buf: &mut [u8]) -> Result<(), Error<R::Error>> {
    let readbuf = reader.fill(1)?;
    let readbuf = readbuf.as_ref();
    if readbuf.len() >= buf.len() {
        buf.copy_from_slice(&readbuf[..buf.len()]);
        reader.advance(1);
        Ok(())
    } else {
        Err(Error::Eof)
    }
}

struct TypeNum {
    major_limit: u8,
    byte: u8
}

impl TypeNum {
    const fn new(major: u8, byte: u8) -> TypeNum {
        TypeNum {
            major_limit: !major,
            byte
        }
    }

    fn decode_u8<'a, R: Read<'a>>(self, reader: &mut R) -> Result<u8, Error<R::Error>> {
        match self.byte & self.major_limit {
            x @ 0 ..= 0x17 => Ok(x),
            0x18 => pull_one(reader),
            _ => Err(Error::mismatch(self.major_limit, self.byte))
        }
    }


    fn decode_u16<'a, R: Read<'a>>(self, reader: &mut R) -> Result<u16, Error<R::Error>> {
        match self.byte & self.major_limit {
            x @ 0 ..= 0x17 => Ok(x.into()),
            0x18 => pull_one(reader).map(Into::into),
            0x19 => {
                let mut buf = [0; 2];
                pull_exact(reader, &mut buf)?;
                Ok(u16::from_be_bytes(buf))
            },
            _ => Err(Error::mismatch(self.major_limit, self.byte))
        }
    }

    fn decode_u32<'a, R: Read<'a>>(self, reader: &mut R) -> Result<u32, Error<R::Error>> {
        match self.byte & self.major_limit {
            x @ 0 ..= 0x17 => Ok(x.into()),
            0x18 => pull_one(reader).map(Into::into),
            0x19 => {
                let mut buf = [0; 2];
                pull_exact(reader, &mut buf)?;
                Ok(u16::from_be_bytes(buf).into())
            },
            0x1a => {
                let mut buf = [0; 4];
                pull_exact(reader, &mut buf)?;
                Ok(u32::from_be_bytes(buf))
            }
            _ => Err(Error::mismatch(self.major_limit, self.byte))
        }
    }

    fn decode_u64<'a, R: Read<'a>>(self, reader: &mut R) -> Result<u64, Error<R::Error>> {
        match self.byte & self.major_limit {
            x @ 0 ..= 0x17 => Ok(x.into()),
            0x18 => pull_one(reader).map(Into::into),
            0x19 => {
                let mut buf = [0; 2];
                pull_exact(reader, &mut buf)?;
                Ok(u16::from_be_bytes(buf).into())
            },
            0x1a => {
                let mut buf = [0; 4];
                pull_exact(reader, &mut buf)?;
                Ok(u32::from_be_bytes(buf).into())
            },
            0x1b => {
                let mut buf = [0; 8];
                pull_exact(reader, &mut buf)?;
                Ok(u64::from_be_bytes(buf))
            },
            _ => return Err(Error::mismatch(self.major_limit, self.byte))
        }
    }
}

macro_rules! decode_ux {
    ( $( $t:ty , $decode_fn:ident );* $( ; )? ) => {
        $(
            impl<'a> Decode<'a> for $t {
                fn decode<R: Read<'a>>(reader: &mut R) -> Result<Self, Error<R::Error>> {
                    let b = pull_one(reader)?;
                    TypeNum::new(!(major::UNSIGNED << 5), b).$decode_fn(reader)
                }
            }
        )*
    }
}

macro_rules! decode_ix {
    ( $( $t:ty , $decode_fn:ident );* $( ; )? ) => {
        $(
            impl<'a> Decode<'a> for $t {
                fn decode<R: Read<'a>>(reader: &mut R) -> Result<Self, Error<R::Error>> {
                    let b = pull_one(reader)?;
                    match b >> 5 {
                        major::UNSIGNED => {
                            let v = TypeNum::new(!(major::UNSIGNED << 5), b).$decode_fn(reader)?;
                            <$t>::try_from(v).map_err(Error::CastOverflow)
                        },
                        major::NEGATIVE => {
                            let v = TypeNum::new(!(major::NEGATIVE << 5), b).$decode_fn(reader)?;
                            let v = v.checked_add(1)
                                .ok_or(Error::Overflow { name: stringify!($t) })?;
                            let v = <$t>::try_from(v)
                                .map_err(Error::CastOverflow)?;
                            Ok(-v)
                        },
                        _ => Err(Error::TypeMismatch {
                            name: stringify!($t),
                            byte: b
                        })
                    }
                }
            }
        )*
    }
}

decode_ux! {
    u8, decode_u8;
    u16, decode_u16;
    u32, decode_u32;
    u64, decode_u64;
}

decode_ix! {
    i8, decode_u8;
    i16, decode_u16;
    i32, decode_u32;
    i64, decode_u64;
}

struct TypeBytes<'a, const TYPE: u8>(&'a [u8]);

#[cfg(feature = "use_alloc")]
struct TypeBuf<const TYPE: u8>(Vec<u8>);

impl<'a, const TYPE: u8> Decode<'a> for TypeBytes<'a, TYPE> {
    fn decode<R: Read<'a>>(reader: &mut R) -> Result<Self, Error<R::Error>> {
        let b = pull_one(reader)?;
        let len = TypeNum::new(TYPE, b).decode_u64(reader)?;
        let len = usize::try_from(len).map_err(Error::CastOverflow)?;

        match reader.fill(len)? {
            Reference::Long(buf)
                if buf.len() >= len => Ok(TypeBytes(&buf[..len])),
            Reference::Long(buf) => Err(Error::RequireLength {
                name: "bytes",
                expect: len,
                value: buf.len()
            }),
            Reference::Short(_) => Err(Error::RequireBorrowed { name: "bytes" })
        }
    }
}

// TODO support bytes seq
#[cfg(feature = "use_alloc")]
impl<'a, const TYPE: u8> Decode<'a> for TypeBuf<TYPE> {
    fn decode<R: Read<'a>>(reader: &mut R) -> Result<Self, Error<R::Error>> {
        const CAP_LIMIT: usize = 16 * 1024;

        let b = pull_one(reader)?;
        let len = TypeNum::new(TYPE, b).decode_u64(reader)?;
        let mut len = usize::try_from(len).map_err(Error::CastOverflow)?;

        let mut buf = if len <= CAP_LIMIT {
            Vec::with_capacity(len)
        } else {
            Vec::new()
        };

        while len != 0 {
            let readbuf = reader.fill(len)?;
            buf.extend_from_slice(readbuf.as_ref());
            let readlen = readbuf.as_ref().len();
            reader.advance(readlen);
            len -= readlen;

            // TODO try_reserve ?
        }

        Ok(TypeBuf(buf))
    }
}

impl<'a> Decode<'a> for types::Bytes<&'a [u8]> {
    fn decode<R: Read<'a>>(reader: &mut R) -> Result<Self, Error<R::Error>> {
        let buf = TypeBytes::<{ !(major::BYTES << 5) }>::decode(reader)?;
        Ok(types::Bytes(buf.0))
    }
}

#[cfg(feature = "use_alloc")]
impl<'a> Decode<'a> for types::Bytes<Vec<u8>> {
    fn decode<R: Read<'a>>(reader: &mut R) -> Result<Self, Error<R::Error>> {
        let buf = TypeBuf::<{ !(major::BYTES << 5) }>::decode(reader)?;
        Ok(types::Bytes(buf.0))
    }
}

impl<'a> Decode<'a> for &'a str {
    fn decode<R: Read<'a>>(reader: &mut R) -> Result<Self, Error<R::Error>> {
        let buf = TypeBytes::<{ !(major::STRING << 5) }>::decode(reader)?;
        core::str::from_utf8(buf.0).map_err(Error::InvalidUtf8)
    }
}

#[cfg(feature = "use_alloc")]
impl<'a> Decode<'a> for types::Bytes<String> {
    fn decode<R: Read<'a>>(reader: &mut R) -> Result<Self, Error<R::Error>> {
        let buf = TypeBuf::<{ !(major::STRING << 5) }>::decode(reader)?;
        let buf = String::from_utf8(buf.0)
            .map_err(|err| Error::InvalidUtf8(err.utf8_error()))?;
        Ok(types::Bytes(buf))
    }
}

impl<'a> Decode<'a> for types::BadStr<&'a [u8]> {
    fn decode<R: Read<'a>>(reader: &mut R) -> Result<Self, Error<R::Error>> {
        let buf = TypeBytes::<{ !(major::STRING << 5) }>::decode(reader)?;
        Ok(types::BadStr(buf.0))
    }
}

#[cfg(feature = "use_alloc")]
impl<'a> Decode<'a> for types::BadStr<Vec<u8>> {
    fn decode<R: Read<'a>>(reader: &mut R) -> Result<Self, Error<R::Error>> {
        let buf = TypeBuf::<{ !(major::STRING << 5) }>::decode(reader)?;
        Ok(types::BadStr(buf.0))
    }
}

#[cfg(feature = "use_alloc")]
impl<'a, T: Decode<'a>> Decode<'a> for Vec<T> {
    fn decode<R: Read<'a>>(reader: &mut R) -> Result<Self, Error<R::Error>> {
        let b = pull_one(reader)?;
        // TypeNum::new(!(major::ARRAY << 5), b).decode_u64(reader)?;
        todo!()
    }
}


pub enum Token {
    Unsigned(u8),
    Negative(u8),
    Bytes(u8),
    String(u8),
    Array(u8),
    Map(u8),
    Tag(u8),
    Simple(u8),
}

pub enum Size {
    U8,
    U16,
    U32,
    U64
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
    Tag(u64)
}

macro_rules! lookup {
    (
        static $name:ident = [$ty:ty ; $size:expr];
        $( $( $namespace:ident :: $token:ident )|* => $val:expr ,)*
        _ => $default:expr $(,)?
    ) => (
        static $name: [$ty; $size] = {
            let default = $default as $ty;
            let mut table = [default; $size];

            $(
                let val = $val as $ty;
                $(
                    table[$namespace :: $token as usize] = val;
                )*
            )*

            table
        };
    )
}

impl Token {
    fn parse(x: u8) -> Option<Token> {
        type Parser = fn(u8) -> Option<Token>;

        lookup! {
            static LUT = [Parser; 8];

            major::UNSIGNED => |x| None,
            _ => |x| None
        }

        LUT.get((x >> 5) as usize)?(x)
    }

    fn want(&self) -> usize {
        todo!()
    }

    fn read(&self, input: &[u8]) -> Type {
        todo!()
    }
}
