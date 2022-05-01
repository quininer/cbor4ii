//! decode module

use core::convert::TryFrom;
use crate::core::{ major, marker, types };
use crate::util::ScopeGuard;
pub use crate::error::DecodeError as Error;

#[cfg(feature = "use_alloc")]
use crate::alloc::{ vec::Vec, string::String };


/// Read trait
///
/// This is similar to `BufRead` of standard library,
/// but can define its own error types, and can get a
/// reference with a long enough lifetime to implement zero-copy decode.
pub trait Read<'de> {
    #[cfg(feature = "use_std")]
    type Error: std::error::Error + 'static;

    #[cfg(not(feature = "use_std"))]
    type Error: core::fmt::Display + core::fmt::Debug;

    /// Returns the available bytes.
    ///
    /// The want value is the expected value.
    /// If the length of bytes returned is less than this value,
    /// zero-copy decoding will not be possible.
    ///
    /// Returning empty bytes means EOF.
    fn fill<'short>(&'short mut self, want: usize) -> Result<Reference<'de, 'short>, Self::Error>;

    /// Advance reader
    fn advance(&mut self, n: usize);

    /// Step count
    ///
    /// This method maybe called when the decode is started
    /// to calculate the decode depth.
    /// If it returns false, the decode will return a depth limit error.
    #[inline]
    fn step_in(&mut self) -> bool {
        true
    }

    /// Step count
    ///
    /// This method maybe called when the decode is completed
    /// to calculate the decode depth.
    #[inline]
    fn step_out(&mut self) {}
}

/// Bytes reference
pub enum Reference<'de, 'short> {
    /// If the reader can return bytes as long as its lifetime,
    /// then zero-copy decoding will be allowed.
    Long(&'de [u8]),

    /// Bytes returned normally
    Short(&'short [u8])
}

/// Decode trait
pub trait Decode<'a>: Sized {
    /// Decode to type
    fn decode<R: Read<'a>>(reader: &mut R) -> Result<Self, Error<R::Error>>;
}

impl Reference<'_, '_> {
    #[inline]
    pub const fn as_ref(&self) -> &[u8] {
        match self {
            Reference::Long(buf) => buf,
            Reference::Short(buf) => buf
        }
    }
}

impl<'a, 'de, T: Read<'de>> Read<'de> for &'a mut T {
    type Error = T::Error;

    #[inline]
    fn fill<'short>(&'short mut self, want: usize) -> Result<Reference<'de, 'short>, Self::Error> {
        (**self).fill(want)
    }

    #[inline]
    fn advance(&mut self, n: usize) {
        (**self).advance(n)
    }

    #[inline]
    fn step_in(&mut self) -> bool {
        (**self).step_in()
    }

    #[inline]
    fn step_out(&mut self) {
        (**self).step_out()
    }
}

#[inline]
#[cfg_attr(not(feature = "serde1"), allow(dead_code))]
pub(crate) fn peek_one<'a, R: Read<'a>>(reader: &mut R) -> Result<u8, Error<R::Error>> {
    let b = reader.fill(1)?
        .as_ref()
        .get(0)
        .copied()
        .ok_or(Error::Eof)?;
    Ok(b)
}

#[inline]
pub(crate) fn pull_one<'a, R: Read<'a>>(reader: &mut R) -> Result<u8, Error<R::Error>> {
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

#[inline]
fn skip_exact<'de, R: Read<'de>>(reader: &mut R, mut len: usize) -> Result<(), R::Error> {
    while len != 0 {
        let buf = reader.fill(len)?;
        let buf = buf.as_ref();

        let buflen = core::cmp::min(len, buf.len());
        reader.advance(buflen);
        len -= buflen;
    }

    Ok(())
}

pub(crate) struct TypeNum {
    major_limit: u8
}

impl TypeNum {
    #[inline]
    pub(crate) const fn new(major: u8) -> TypeNum {
        TypeNum { major_limit: !(major << 5) }
    }

    #[inline]
    pub fn decode_u8<'a, R: Read<'a>>(self, reader: &mut R) -> Result<u8, Error<R::Error>> {
        let byte = pull_one(reader)?;
        match byte & self.major_limit {
            x @ 0 ..= 0x17 => Ok(x),
            0x18 => pull_one(reader),
            _ => Err(Error::mismatch(self.major_limit, byte))
        }
    }

    #[inline]
    fn decode_u16<'a, R: Read<'a>>(self, reader: &mut R) -> Result<u16, Error<R::Error>> {
        let byte = pull_one(reader)?;
        match byte & self.major_limit {
            x @ 0 ..= 0x17 => Ok(x.into()),
            0x18 => pull_one(reader).map(Into::into),
            0x19 => {
                let mut buf = [0; 2];
                pull_exact(reader, &mut buf)?;
                Ok(u16::from_be_bytes(buf))
            },
            _ => Err(Error::mismatch(self.major_limit, byte))
        }
    }

    #[inline]
    fn decode_u32<'a, R: Read<'a>>(self, reader: &mut R) -> Result<u32, Error<R::Error>> {
        let byte = pull_one(reader)?;
        match byte & self.major_limit {
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
            _ => Err(Error::mismatch(self.major_limit, byte))
        }
    }

    #[inline]
    pub(crate) fn decode_u64<'a, R: Read<'a>>(self, reader: &mut R) -> Result<u64, Error<R::Error>> {
        let byte = pull_one(reader)?;
        match byte & self.major_limit {
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
            _ => Err(Error::mismatch(self.major_limit, byte))
        }
    }
}

macro_rules! decode_ux {
    ( $( $t:ty , $decode_fn:ident );* $( ; )? ) => {
        $(
            impl<'a> Decode<'a> for $t {
                #[inline]
                fn decode<R: Read<'a>>(reader: &mut R) -> Result<Self, Error<R::Error>> {
                    TypeNum::new(major::UNSIGNED).$decode_fn(reader)
                }
            }
        )*
    }
}

macro_rules! decode_nx {
    ( $( $t:ty , $decode_fn:ident );* $( ; )? ) => {
        $(
            impl<'a> Decode<'a> for types::Negative<$t> {
                #[inline]
                fn decode<R: Read<'a>>(reader: &mut R) -> Result<Self, Error<R::Error>> {
                    TypeNum::new(major::NEGATIVE)
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
                #[inline]
                fn decode<R: Read<'a>>(reader: &mut R) -> Result<Self, Error<R::Error>> {
                    let byte = peek_one(reader)?;
                    match if_major(byte) {
                        major::UNSIGNED => {
                            let v = TypeNum::new(major::UNSIGNED).$decode_fn(reader)?;
                            <$t>::try_from(v).map_err(Error::CastOverflow)
                        },
                        major::NEGATIVE => {
                            let v = TypeNum::new(major::NEGATIVE).$decode_fn(reader)?;
                            let v = <$t>::try_from(v)
                                .map_err(Error::CastOverflow)?;
                            let v = -v;
                            let v = v.checked_sub(1)
                                .ok_or(Error::Overflow { name: stringify!($t) })?;
                            Ok(v)
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

#[inline]
fn decode_x128<'a, R: Read<'a>>(name: &'static str, reader: &mut R) -> Result<[u8; 16], Error<R::Error>> {
    let len = decode_len(major::BYTES, reader)?
        .ok_or(Error::RequireLength { name, expect: 0, value: 0 })?;
    let mut buf = [0; 16];
    if let Some(pos) = buf.len().checked_sub(len) {
        pull_exact(reader, &mut buf[pos..])?;
        Ok(buf)
    } else {
        Err(Error::Overflow { name })
    }
}

impl<'a> Decode<'a> for u128 {
    #[inline]
    fn decode<R: Read<'a>>(reader: &mut R) -> Result<Self, Error<R::Error>> {
        let byte = peek_one(reader)?;
        if if_major(byte) == major::UNSIGNED {
            u64::decode(reader).map(Into::into)
        } else {
            let tag = TypeNum::new(major::TAG).decode_u8(reader)?;
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
    #[inline]
    fn decode<R: Read<'a>>(reader: &mut R) -> Result<Self, Error<R::Error>> {
        let byte = peek_one(reader)?;
        match if_major(byte) {
            major::UNSIGNED => u64::decode(reader).map(Into::into),
            major::NEGATIVE => {
               // Negative numbers can be as big as 2^64, hence decode them as unsigned 64-bit
               // value first.
               let n = TypeNum::new(major::NEGATIVE).decode_u64(reader)?;
               let n = i128::from(n);
               let n = -n;
               let n = n - 1;
               Ok(n)
            },
            _ => {
                let tag = TypeNum::new(major::TAG).decode_u8(reader)?;
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
                        let n = i128::try_from(n).map_err(Error::CastOverflow)?;
                        let n = -n;
                        let n = n.checked_sub(1)
                            .ok_or(Error::Overflow { name: "i128" })?;
                        Ok(n)
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

#[inline]
fn decode_bytes<'a, R: Read<'a>>(name: &'static str, major: u8, reader: &mut R)
    -> Result<&'a [u8], Error<R::Error>>
{
    let len = TypeNum::new(major).decode_u64(reader)?;
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

#[inline]
#[cfg(feature = "use_alloc")]
fn decode_buf<'a, R: Read<'a>>(major: u8, reader: &mut R, buf: &mut Vec<u8>)
    -> Result<Option<&'a [u8]>, Error<R::Error>>
{
    const CAP_LIMIT: usize = 16 * 1024;

    if let Some(mut len) = decode_len(major, reader)? {
        // try long lifetime buffer
        if let Reference::Long(buf) = reader.fill(len)? {
            if buf.len() >= len {
                reader.advance(len);
                return Ok(Some(&buf[..len]));
            }
        }

        buf.reserve(core::cmp::min(len, CAP_LIMIT)); // TODO try_reserve ?

        while len != 0 {
            let readbuf = reader.fill(len)?;
            let readbuf = readbuf.as_ref();

            if readbuf.is_empty() {
                return Err(Error::Eof);
            }

            let readlen = core::cmp::min(readbuf.len(), len);

            buf.extend_from_slice(&readbuf[..readlen]);
            reader.advance(readlen);
            len -= readlen;
        }

        Ok(None)
    } else {
        // bytes sequence
        while !is_break(reader)? {
            if !reader.step_in() {
                return Err(Error::DepthLimit);
            }
            let mut reader = ScopeGuard(reader, |reader| reader.step_out());
            let reader = &mut *reader;

            if let Some(longbuf) = decode_buf(major, reader, buf)? {
                buf.extend_from_slice(longbuf);
            }
        }

        Ok(None)
    }
}

impl<'a> Decode<'a> for types::Bytes<&'a [u8]> {
    #[inline]
    fn decode<R: Read<'a>>(reader: &mut R) -> Result<Self, Error<R::Error>> {
        let buf = decode_bytes("bytes", major::BYTES, reader)?;
        Ok(types::Bytes(buf))
    }
}

#[cfg(feature = "use_alloc")]
impl<'a> Decode<'a> for types::Bytes<Vec<u8>> {
    #[inline]
    fn decode<R: Read<'a>>(reader: &mut R) -> Result<Self, Error<R::Error>> {
        let mut buf = Vec::new();
        if let Some(longbuf) = decode_buf(major::BYTES, reader, &mut buf)? {
            buf.extend_from_slice(longbuf);
        }
        Ok(types::Bytes(buf))
    }
}

#[cfg(feature = "use_alloc")]
impl<'a> Decode<'a> for types::Bytes<crate::alloc::borrow::Cow<'a, [u8]>> {
    #[inline]
    fn decode<R: Read<'a>>(reader: &mut R) -> Result<Self, Error<R::Error>> {
        use crate::alloc::borrow::Cow;

        let mut buf = Vec::new();
        Ok(types::Bytes(if let Some(longbuf) = decode_buf(major::BYTES, reader, &mut buf)? {
            Cow::Borrowed(longbuf)
        } else {
            Cow::Owned(buf)
        }))
    }
}


impl<'a> Decode<'a> for &'a str {
    #[inline]
    fn decode<R: Read<'a>>(reader: &mut R) -> Result<Self, Error<R::Error>> {
        let buf = decode_bytes("str", major::STRING, reader)?;
        core::str::from_utf8(buf).map_err(Error::InvalidUtf8)
    }
}

#[cfg(feature = "use_alloc")]
impl<'a> Decode<'a> for String {
    #[inline]
    fn decode<R: Read<'a>>(reader: &mut R) -> Result<Self, Error<R::Error>> {
        let mut buf = Vec::new();
        if let Some(longbuf) = decode_buf(major::STRING, reader, &mut buf)? {
            buf.extend_from_slice(longbuf);
        }
        let buf = String::from_utf8(buf)
            .map_err(|err| Error::InvalidUtf8(err.utf8_error()))?;
        Ok(buf)
    }
}


#[cfg(feature = "use_alloc")]
impl<'a> Decode<'a> for crate::alloc::borrow::Cow<'a, str> {
    #[inline]
    fn decode<R: Read<'a>>(reader: &mut R) -> Result<Self, Error<R::Error>> {
        use crate::alloc::borrow::Cow;

        let mut buf = Vec::new();
        Ok(if let Some(longbuf) = decode_buf(major::STRING, reader, &mut buf)? {
            Cow::Borrowed(core::str::from_utf8(longbuf).map_err(Error::InvalidUtf8)?)
        } else {
            let buf = String::from_utf8(buf)
                .map_err(|err| Error::InvalidUtf8(err.utf8_error()))?;
            Cow::Owned(buf)
        })
    }
}

impl<'a> Decode<'a> for types::BadStr<&'a [u8]> {
    #[inline]
    fn decode<R: Read<'a>>(reader: &mut R) -> Result<Self, Error<R::Error>> {
        let buf = decode_bytes("str", major::STRING, reader)?;
        Ok(types::BadStr(buf))
    }
}

#[cfg(feature = "use_alloc")]
impl<'a> Decode<'a> for types::BadStr<Vec<u8>> {
    #[inline]
    fn decode<R: Read<'a>>(reader: &mut R) -> Result<Self, Error<R::Error>> {
        let mut buf = Vec::new();
        if let Some(longbuf) = decode_buf(major::STRING, reader, &mut buf)? {
            buf.extend_from_slice(longbuf);
        }
        Ok(types::BadStr(buf))
    }
}

#[cfg(feature = "use_alloc")]
impl<'a> Decode<'a> for types::BadStr<crate::alloc::borrow::Cow<'a, [u8]>> {
    #[inline]
    fn decode<R: Read<'a>>(reader: &mut R) -> Result<Self, Error<R::Error>> {
        use crate::alloc::borrow::Cow;

        let mut buf = Vec::new();
        Ok(types::BadStr(if let Some(longbuf) = decode_buf(major::STRING, reader, &mut buf)? {
            Cow::Borrowed(longbuf)
        } else {
            Cow::Owned(buf)
        }))
    }
}

#[inline]
fn decode_len<'a, R: Read<'a>>(major: u8, reader: &mut R)
    -> Result<Option<usize>, Error<R::Error>>
{
    let byte = peek_one(reader)?;
    if byte != (marker::START | (major << 5)) {
        let len = TypeNum::new(major).decode_u64(reader)?;
        let len = usize::try_from(len).map_err(Error::CastOverflow)?;
        Ok(Some(len))
    } else {
        reader.advance(1);
        Ok(None)
    }
}

pub struct ArrayStart(pub Option<usize>);

impl<'a> Decode<'a> for ArrayStart {
    #[inline]
    fn decode<R: Read<'a>>(reader: &mut R) -> Result<Self, Error<R::Error>> {
        decode_len(major::ARRAY, reader).map(ArrayStart)
    }
}

#[cfg(feature = "use_alloc")]
impl<'a, T: Decode<'a>> Decode<'a> for Vec<T> {
    #[inline]
    fn decode<R: Read<'a>>(reader: &mut R) -> Result<Self, Error<R::Error>> {
        let mut arr = Vec::new();

        if !reader.step_in() {
            return Err(Error::DepthLimit);
        }
        let mut reader = ScopeGuard(reader, |reader| reader.step_out());
        let reader = &mut *reader;

        if let Some(len) = decode_len(major::ARRAY, reader)? {
            arr.reserve(core::cmp::min(len, 256)); // TODO try_reserve ?

            for _ in 0..len {
                let value = T::decode(reader)?;
                arr.push(value);
            }
        } else {
            while !is_break(reader)? {
                let value = T::decode(reader)?;
                arr.push(value);
            }
        }

        Ok(arr)
    }
}

pub struct MapStart(pub Option<usize>);

impl<'a> Decode<'a> for MapStart {
    #[inline]
    fn decode<R: Read<'a>>(reader: &mut R) -> Result<Self, Error<R::Error>> {
        decode_len(major::MAP, reader).map(MapStart)
    }
}

#[cfg(feature = "use_alloc")]
impl<'a, K: Decode<'a>, V: Decode<'a>> Decode<'a> for types::Map<Vec<(K, V)>> {
    #[inline]
    fn decode<R: Read<'a>>(reader: &mut R) -> Result<Self, Error<R::Error>> {
        let mut map = Vec::new();

        if !reader.step_in() {
            return Err(Error::DepthLimit);
        }
        let mut reader = ScopeGuard(reader, |reader| reader.step_out());
        let reader = &mut *reader;

        if let Some(len) = decode_len(major::MAP, reader)? {
            map.reserve(core::cmp::min(len, 256)); // TODO try_reserve ?

            for _ in 0..len {
                let k = K::decode(reader)?;
                let v = V::decode(reader)?;
                map.push((k, v));
            }
        } else {
            while !is_break(reader)? {
                let k = K::decode(reader)?;
                let v = V::decode(reader)?;
                map.push((k, v));
            }
        }

        Ok(types::Map(map))
    }
}

pub struct TagStart(pub u64);

impl<'a> Decode<'a> for TagStart {
    #[inline]
    fn decode<R: Read<'a>>(reader: &mut R) -> Result<Self, Error<R::Error>> {
        TypeNum::new(major::TAG).decode_u64(reader).map(TagStart)
    }
}

impl<'a, T: Decode<'a>> Decode<'a> for types::Tag<T> {
    #[inline]
    fn decode<R: Read<'a>>(reader: &mut R) -> Result<Self, Error<R::Error>> {
        let tag = TypeNum::new(major::TAG).decode_u64(reader)?;
        let value = T::decode(reader)?;
        Ok(types::Tag(tag, value))
    }
}

impl<'a> Decode<'a> for types::Simple {
    #[inline]
    fn decode<R: Read<'a>>(reader: &mut R) -> Result<Self, Error<R::Error>> {
        let n = TypeNum::new(major::SIMPLE).decode_u8(reader)?;
        Ok(types::Simple(n))
    }
}

impl<'a> Decode<'a> for bool {
    #[inline]
    fn decode<R: Read<'a>>(reader: &mut R) -> Result<Self, Error<R::Error>> {
        let byte = pull_one(reader)?;
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
    #[inline]
    fn decode<R: Read<'a>>(reader: &mut R) -> Result<Self, Error<R::Error>> {
        let byte = peek_one(reader)?;
        if byte != marker::NULL && byte != marker::UNDEFINED {
            T::decode(reader).map(Some)
        } else {
            reader.advance(1);
            Ok(None)
        }
    }
}

impl<'a> Decode<'a> for types::F16 {
    #[inline]
    fn decode<R: Read<'a>>(reader: &mut R) -> Result<Self, Error<R::Error>> {
        let byte = peek_one(reader)?;
        if byte == marker::F16 {
            reader.advance(1);
            let mut buf = [0; 2];
            pull_exact(reader, &mut buf)?;
            Ok(types::F16(u16::from_be_bytes(buf)))
        } else {
            Err(Error::TypeMismatch {
                name: "f16",
                byte
            })
        }
    }
}

#[cfg(feature = "half-f16")]
impl<'a> Decode<'a> for half::f16 {
    #[inline]
    fn decode<R: Read<'a>>(reader: &mut R) -> Result<Self, Error<R::Error>> {
        let types::F16(n) = types::F16::decode(reader)?;
        Ok(half::f16::from_bits(n))
    }
}

impl<'a> Decode<'a> for f32 {
    #[inline]
    fn decode<R: Read<'a>>(reader: &mut R) -> Result<Self, Error<R::Error>> {
        let byte = peek_one(reader)?;
        if byte == marker::F32 {
            reader.advance(1);
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
    #[inline]
    fn decode<R: Read<'a>>(reader: &mut R) -> Result<Self, Error<R::Error>> {
        let byte = peek_one(reader)?;
        if byte == marker::F64 {
            reader.advance(1);
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

/// Ignore an arbitrary object
pub struct IgnoredAny;

impl<'a> Decode<'a> for IgnoredAny {
    fn decode<R: Read<'a>>(reader: &mut R) -> Result<Self, Error<R::Error>> {
        if !reader.step_in() {
            return Err(Error::DepthLimit);
        }
        let mut reader = ScopeGuard(reader, |reader| reader.step_out());
        let reader = &mut *reader;

        let byte = peek_one(reader)?;

        match if_major(byte) {
            major @ major::UNSIGNED | major @ major::NEGATIVE => {
                let skip = match byte & !(major << 5) {
                    0 ..= 0x17 => 0,
                    0x18 => 1,
                    0x19 => 2,
                    0x1a => 4,
                    0x1b => 8,
                    _ => return Err(Error::TypeMismatch {
                        name: "ignore-any",
                        byte
                    })
                };
                skip_exact(reader, skip + 1)?;
            },
            major @ major::BYTES | major @ major::STRING |
            major @ major::ARRAY | major @ major::MAP => {
                if let Some(len) = decode_len(major, reader)? {
                    match major {
                        major::BYTES | major::STRING => skip_exact(reader, len)?,
                        major::ARRAY | major::MAP => for _ in 0..len {
                            let _ignore = IgnoredAny::decode(reader)?;

                            if major == major::MAP {
                                let _ignore = IgnoredAny::decode(reader)?;
                            }
                        },
                        _ => ()
                    }
                } else {
                    while !is_break(reader)? {
                        let _ignore = IgnoredAny::decode(reader)?;

                        if major == major::MAP {
                            let _ignore = IgnoredAny::decode(reader)?;
                        }
                    }
                }
            },
            major @ major::TAG => {
                let _tag = TypeNum::new(major).decode_u64(reader)?;
                let _ignore = IgnoredAny::decode(reader)?;
            },
            major::SIMPLE => {
                let skip = match byte {
                    marker::FALSE
                        | marker::TRUE
                        | marker::NULL
                        | marker::UNDEFINED => 0,
                    marker::F16 => 2,
                    marker::F32 => 4,
                    marker::F64 => 8,
                    _ => return Err(Error::Unsupported { byte })
                };
                skip_exact(reader, skip + 1)?;
            },
            _ => return Err(Error::Unsupported { byte })
        }

        Ok(IgnoredAny)
    }
}

#[inline]
pub fn is_break<'a, R: Read<'a>>(reader: &mut R) -> Result<bool, Error<R::Error>> {
    if peek_one(reader)? == marker::BREAK {
        reader.advance(1);
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Determine the object type from the given byte.
#[inline]
pub fn if_major(byte: u8) -> u8 {
    byte >> 5
}
