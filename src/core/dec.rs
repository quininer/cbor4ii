use crate::core::{ major, marker, types };
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

    fn step_in(&mut self) -> bool {
        true
    }

    fn step_out(&mut self) {}
}

pub enum Reference<'a, 'b> {
    Long(&'a [u8]),
    Short(&'b [u8])
}

pub trait Decode<'a>: Sized {
    fn decode_with<R: Read<'a>>(byte: u8, reader: &mut R) -> Result<Self, Error<R::Error>>;

    fn decode<R: Read<'a>>(reader: &mut R) -> Result<Self, Error<R::Error>> {
        let byte = pull_one(reader)?;
        Self::decode_with(byte, reader)
    }
}

impl Reference<'_, '_> {
    #[inline]
    pub(crate) const fn as_ref(&self) -> &[u8] {
        match self {
            Reference::Long(buf) => buf,
            Reference::Short(buf) => buf
        }
    }
}

#[inline]
pub fn peek_one<'a, R: Read<'a>>(reader: &mut R) -> Result<u8, Error<R::Error>> {
    let b = reader.fill(1)?
        .as_ref()
        .get(0)
        .copied()
        .ok_or(Error::Eof)?;
    Ok(b)
}

#[inline]
pub fn pull_one<'a, R: Read<'a>>(reader: &mut R) -> Result<u8, Error<R::Error>> {
    let b = reader.fill(1)?
        .as_ref()
        .get(0)
        .copied()
        .ok_or(Error::Eof)?;
    reader.advance(1);
    Ok(b)
}

#[inline]
fn pull_exact<'a, R: Read<'a>>(reader: &mut R, mut buf: &mut [u8]) -> Result<(), Error<R::Error>> {
    while !buf.is_empty() {
        let readbuf = reader.fill(buf.len())?;
        let readbuf = readbuf.as_ref();

        if readbuf.is_empty() {
            return Err(Error::Eof);
        }

        let len = core::cmp::min(buf.len(), readbuf.len());
        buf[..len].copy_from_slice(&readbuf[..len]);
        reader.advance(len);
        buf = &mut buf[len..];
    }

    Ok(())
}

struct TypeNum {
    major_limit: u8,
    byte: u8
}

impl TypeNum {
    const fn new(major_limit: u8, byte: u8) -> TypeNum {
        TypeNum { major_limit, byte }
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
            _ => Err(Error::mismatch(self.major_limit, self.byte))
        }
    }
}

macro_rules! decode_ux {
    ( $( $t:ty , $decode_fn:ident );* $( ; )? ) => {
        $(
            impl<'a> Decode<'a> for $t {
                fn decode_with<R: Read<'a>>(byte: u8, reader: &mut R) -> Result<Self, Error<R::Error>> {
                    TypeNum::new(!(major::UNSIGNED << 5), byte).$decode_fn(reader)
                }
            }
        )*
    }
}

macro_rules! decode_nx {
    ( $( $t:ty , $decode_fn:ident );* $( ; )? ) => {
        $(
            impl<'a> Decode<'a> for types::Negative<$t> {
                fn decode_with<R: Read<'a>>(byte: u8, reader: &mut R) -> Result<Self, Error<R::Error>> {
                    TypeNum::new(!(major::NEGATIVE << 5), byte)
                        .$decode_fn(reader)
                        .map(types::Negative)
                }
            }
        )*
    }

}

macro_rules! decode_ix {
    ( $( $t:ty , $decode_fn:ident );* $( ; )? ) => {
        $(
            impl<'a> Decode<'a> for $t {
                fn decode_with<R: Read<'a>>(byte: u8, reader: &mut R) -> Result<Self, Error<R::Error>> {
                    match byte >> 5 {
                        major::UNSIGNED => {
                            let v = TypeNum::new(!(major::UNSIGNED << 5), byte).$decode_fn(reader)?;
                            <$t>::try_from(v).map_err(Error::CastOverflow)
                        },
                        major::NEGATIVE => {
                            let v = TypeNum::new(!(major::NEGATIVE << 5), byte).$decode_fn(reader)?;
                            let v = v.checked_add(1)
                                .ok_or(Error::Overflow { name: stringify!($t) })?;
                            let v = <$t>::try_from(v)
                                .map_err(Error::CastOverflow)?;
                            Ok(-v)
                        },
                        _ => Err(Error::TypeMismatch {
                            name: stringify!($t),
                            byte
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

decode_nx! {
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

fn decode_x128<'a, R: Read<'a>>(name: &'static str, reader: &mut R) -> Result<[u8; 16], Error<R::Error>> {
    let byte = pull_one(reader)?;
    let len = decode_len(major::BYTES, byte, reader)?
        .ok_or(Error::TypeMismatch { name, byte })?;
    let mut buf = [0; 16];
    if let Some(pos) = buf.len().checked_sub(len) {
        pull_exact(reader, &mut buf[pos..])?;
        Ok(buf)
    } else {
        Err(Error::Overflow { name })
    }
}

impl<'a> Decode<'a> for u128 {
    fn decode_with<R: Read<'a>>(byte: u8, reader: &mut R) -> Result<Self, Error<R::Error>> {
        if byte >> 5 == major::UNSIGNED {
            u64::decode_with(byte, reader).map(Into::into)
        } else {
            let tag = TypeNum::new(!(major::TAG << 5), byte).decode_u8(reader)?;
            if tag == 2 {
                let buf = decode_x128("u128::bytes", reader)?;
                Ok(u128::from_be_bytes(buf))
            } else {
                Err(Error::TypeMismatch {
                    name: "u128",
                    byte: tag
                })
            }
        }
    }
}

impl<'a> Decode<'a> for i128 {
    fn decode_with<R: Read<'a>>(byte: u8, reader: &mut R) -> Result<Self, Error<R::Error>> {
        match byte >> 5 {
            major::UNSIGNED => u64::decode_with(byte, reader).map(Into::into),
            major::NEGATIVE => i64::decode_with(byte, reader).map(Into::into),
            _ => {
                let tag = TypeNum::new(!(major::TAG << 5), byte).decode_u8(reader)?;
                match tag {
                    2 => {
                        let buf = decode_x128("i128<positive>::bytes", reader)?;
                        let n = u128::from_be_bytes(buf);
                        let n = i128::try_from(n).map_err(Error::CastOverflow)?;
                        Ok(n)
                    },
                    3 => {
                        let buf = decode_x128("i128<negative>::bytes", reader)?;
                        let n = u128::from_be_bytes(buf);
                        let n = n.checked_add(1)
                            .ok_or(Error::Overflow { name: "i128" })?;
                        let n = i128::try_from(n).map_err(Error::CastOverflow)?;
                        Ok(-n)
                    },
                    _ => Err(Error::TypeMismatch {
                        name: "i128",
                        byte: tag
                    })
                }
            }
        }
    }
}

fn decode_bytes<'a, R: Read<'a>>(name: &'static str, major_limit: u8, byte: u8, reader: &mut R)
    -> Result<&'a [u8], Error<R::Error>>
{
    let len = TypeNum::new(major_limit, byte).decode_u64(reader)?;
    let len = usize::try_from(len).map_err(Error::CastOverflow)?;

    match reader.fill(len)? {
        Reference::Long(buf) if buf.len() >= len => {
            reader.advance(len);
            Ok(&buf[..len])
        },
        Reference::Long(buf) => Err(Error::RequireLength {
            name,
            expect: len,
            value: buf.len()
        }),
        Reference::Short(_) => Err(Error::RequireBorrowed { name })
    }
}

#[cfg(feature = "use_alloc")]
fn decode_buf<'a, R: Read<'a>>(major: u8, byte: u8, follow: bool, reader: &mut R, buf: &mut Vec<u8>)
    -> Result<(), Error<R::Error>>
{
    const CAP_LIMIT: usize = 16 * 1024;

    if follow && byte == marker::BREAK {
        Ok(())
    } else if let Some(mut len) = decode_len(major, byte, reader)? {
        if len <= CAP_LIMIT {
            buf.reserve(len); // TODO try_reserve ?
        }

        while len != 0 {
            let readbuf = reader.fill(len)?;
            let readbuf = readbuf.as_ref();
            let readlen = readbuf.len();

            if readlen == 0 {
                return Err(Error::Eof);
            }

            buf.extend_from_slice(readbuf);
            reader.advance(readlen);
            len -= readlen;
        }

        Ok(())
    } else {
        let byte = pull_one(reader)?;
        decode_buf(major, byte, true, reader, buf)
    }
}

impl<'a> Decode<'a> for types::Bytes<&'a [u8]> {
    fn decode_with<R: Read<'a>>(byte: u8, reader: &mut R) -> Result<Self, Error<R::Error>> {
        let buf = decode_bytes("bytes", !(major::BYTES << 5), byte, reader)?;
        Ok(types::Bytes(buf))
    }
}

#[cfg(feature = "use_alloc")]
impl<'a> Decode<'a> for types::Bytes<Vec<u8>> {
    fn decode_with<R: Read<'a>>(byte: u8, reader: &mut R) -> Result<Self, Error<R::Error>> {
        let mut buf = Vec::new();
        decode_buf(major::BYTES, byte, false, reader, &mut buf)?;
        Ok(types::Bytes(buf))
    }
}

impl<'a> Decode<'a> for &'a str {
    fn decode_with<R: Read<'a>>(byte: u8, reader: &mut R) -> Result<Self, Error<R::Error>> {
        let buf = decode_bytes("str", !(major::STRING << 5), byte, reader)?;
        core::str::from_utf8(buf).map_err(Error::InvalidUtf8)
    }
}

#[cfg(feature = "use_alloc")]
impl<'a> Decode<'a> for String {
    fn decode_with<R: Read<'a>>(byte: u8, reader: &mut R) -> Result<Self, Error<R::Error>> {
        let mut buf = Vec::new();
        decode_buf(major::STRING, byte, false, reader, &mut buf)?;
        let buf = String::from_utf8(buf)
            .map_err(|err| Error::InvalidUtf8(err.utf8_error()))?;
        Ok(buf)
    }
}

impl<'a> Decode<'a> for types::BadStr<&'a [u8]> {
    fn decode_with<R: Read<'a>>(byte: u8, reader: &mut R) -> Result<Self, Error<R::Error>> {
        let buf = decode_bytes("str", !(major::STRING << 5), byte, reader)?;
        Ok(types::BadStr(buf))
    }
}

#[cfg(feature = "use_alloc")]
impl<'a> Decode<'a> for types::BadStr<Vec<u8>> {
    fn decode_with<R: Read<'a>>(byte: u8, reader: &mut R) -> Result<Self, Error<R::Error>> {
        let mut buf = Vec::new();
        decode_buf(major::STRING, byte, false, reader, &mut buf)?;
        Ok(types::BadStr(buf))
    }
}

pub fn decode_len<'a, R: Read<'a>>(major: u8, byte: u8, reader: &mut R)
    -> Result<Option<usize>, Error<R::Error>>
{
    if byte != (marker::START | (major << 5)) {
        let len = TypeNum::new(!(major << 5), byte).decode_u64(reader)?;
        let len = usize::try_from(len).map_err(Error::CastOverflow)?;
        Ok(Some(len))
    } else {
        Ok(None)
    }
}

#[cfg(feature = "use_alloc")]
impl<'a, T: Decode<'a>> Decode<'a> for Vec<T> {
    fn decode_with<R: Read<'a>>(byte: u8, reader: &mut R) -> Result<Self, Error<R::Error>> {
        let mut arr = Vec::new();

        if let Some(len) = decode_len(major::ARRAY, byte, reader)? {
            for _ in 0..len {
                let value = T::decode(reader)?;
                arr.push(value);
            }
        } else {
            loop {
                let byte = pull_one(reader)?;

                if byte == marker::BREAK {
                    break;
                }

                let value = T::decode_with(byte, reader)?;
                arr.push(value);
            }
        }

        Ok(arr)
    }
}

#[cfg(feature = "use_alloc")]
impl<'a, K: Decode<'a>, V: Decode<'a>> Decode<'a> for types::Map<Vec<(K, V)>> {
    fn decode_with<R: Read<'a>>(byte: u8, reader: &mut R) -> Result<Self, Error<R::Error>> {
        let mut map = Vec::new();

        if let Some(len) = decode_len(major::MAP, byte, reader)? {
            for _ in 0..len {
                let k = K::decode(reader)?;
                let v = V::decode(reader)?;
                map.push((k, v));
            }
        } else {
            loop {
                let byte = pull_one(reader)?;

                if byte == marker::BREAK {
                    break;
                }

                let k = K::decode_with(byte, reader)?;
                let v = V::decode(reader)?;
                map.push((k, v));
            }
        }

        Ok(types::Map(map))
    }
}

impl<'a> Decode<'a> for bool {
    fn decode_with<R: Read<'a>>(byte: u8, _reader: &mut R) -> Result<Self, Error<R::Error>> {
        match byte {
            marker::FALSE => Ok(false),
            marker::TRUE => Ok(true),
            _ => Err(Error::TypeMismatch {
                name: "bool",
                byte
            })
        }
    }
}

impl<'a, T: Decode<'a>> Decode<'a> for Option<T> {
    fn decode_with<R: Read<'a>>(byte: u8, reader: &mut R) -> Result<Self, Error<R::Error>> {
        if byte != marker::NULL && byte != marker::UNDEFINED {
            T::decode_with(byte, reader).map(Some)
        } else {
            Ok(None)
        }
    }
}

#[cfg(feature = "half-f16")]
impl<'a> Decode<'a> for half::f16 {
    fn decode_with<R: Read<'a>>(byte: u8, reader: &mut R) -> Result<Self, Error<R::Error>> {
        if byte == marker::F16 {
            let mut buf = [0; 2];
            pull_exact(reader, &mut buf)?;
            Ok(half::f16::from_be_bytes(buf))
        } else {
            Err(Error::TypeMismatch {
                name: "f16",
                byte
            })
        }
    }
}

impl<'a> Decode<'a> for f32 {
    fn decode_with<R: Read<'a>>(byte: u8, reader: &mut R) -> Result<Self, Error<R::Error>> {
        if byte == marker::F32 {
            let mut buf = [0; 4];
            pull_exact(reader, &mut buf)?;
            Ok(f32::from_be_bytes(buf))
        } else {
            Err(Error::TypeMismatch {
                name: "f32",
                byte
            })
        }
    }
}

impl<'a> Decode<'a> for f64 {
    fn decode_with<R: Read<'a>>(byte: u8, reader: &mut R) -> Result<Self, Error<R::Error>> {
        if byte == marker::F64 {
            let mut buf = [0; 8];
            pull_exact(reader, &mut buf)?;
            Ok(f64::from_be_bytes(buf))
        } else {
            Err(Error::TypeMismatch {
                name: "f64",
                byte
            })
        }
    }
}
