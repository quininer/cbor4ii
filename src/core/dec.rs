//! decode module

use core::convert::TryFrom;
use crate::core::{ major, marker, types, error };
use crate::util::ScopeGuard;
pub use crate::core::error::DecodeError as Error;

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
pub trait Decode<'de>: Sized {
    /// Decode to type
    fn decode<R: Read<'de>>(reader: &mut R) -> Result<Self, Error<R::Error>>;
}

impl<'de, 'short> Reference<'de, 'short> {
    #[inline]
    pub const fn as_ref(&self) -> &[u8] {
        match self {
            Reference::Long(buf) => buf,
            Reference::Short(buf) => buf
        }
    }

    #[inline]
    pub fn take(&self, len: usize) -> Reference<'de, 'short> {
        let len = core::cmp::min(self.as_ref().len(), len);
        match self {
            Reference::Long(buf) => Reference::Long(&buf[..len]),
            Reference::Short(buf) => Reference::Short(&buf[..len])
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
pub(crate) fn peek_one<'de, R: Read<'de>>(name: error::StaticStr, reader: &mut R)
    -> Result<u8, Error<R::Error>>
{
    let b = reader.fill(1)?
        .as_ref()
        .get(0)
        .copied()
        .ok_or_else(|| Error::eof(name, 1))?;
    Ok(b)
}

#[inline]
pub(crate) fn pull_one<'de, R: Read<'de>>(name: error::StaticStr, reader: &mut R)
    -> Result<u8, Error<R::Error>>
{
    let b = peek_one(name, reader)?;
    reader.advance(1);
    Ok(b)
}

#[inline]
fn pull_exact<'de, R: Read<'de>>(name: error::StaticStr, reader: &mut R, mut buf: &mut [u8])
    -> Result<(), Error<R::Error>>
{
    let buf_len = buf.len();
    while !buf.is_empty() {
        let readbuf = reader.fill(buf.len())?;
        let readbuf = readbuf.as_ref();

        if readbuf.is_empty() {
            return Err(Error::eof(name, buf_len));
        }

        let len = core::cmp::min(buf.len(), readbuf.len());
        buf[..len].copy_from_slice(&readbuf[..len]);
        reader.advance(len);
        buf = &mut buf[len..];
    }

    Ok(())
}

#[inline]
fn skip_exact<'de, R: Read<'de>>(reader: &mut R, mut len: usize) -> Result<(), Error<R::Error>> {
    while len != 0 {
        let buf = reader.fill(len)?;
        let buf = buf.as_ref();

        if buf.is_empty() {
            return Err(Error::Eof);
        }

        let buflen = core::cmp::min(len, buf.len());
        reader.advance(buflen);
        len -= buflen;
    }

    Ok(())
}

pub(crate) struct TypeNum {
    name: error::StaticStr,
    major_limit: u8
}

impl TypeNum {
    #[inline]
    pub(crate) const fn new(name: error::StaticStr, major: u8) -> TypeNum {
        TypeNum { name, major_limit: !(major << 5) }
    }

    #[inline]
    pub fn decode_u8<'de, R: Read<'de>>(self, reader: &mut R) -> Result<u8, Error<R::Error>> {
        let byte = pull_one(self.name, reader)?;
        match byte & self.major_limit {
            x @ 0 ..= 0x17 => Ok(x),
            0x18 => pull_one(self.name, reader),
            _ => Err(Error::mismatch(self.name, byte))
        }
    }

    #[inline]
    fn decode_u16<'de, R: Read<'de>>(self, reader: &mut R) -> Result<u16, Error<R::Error>> {
        let byte = pull_one(self.name, reader)?;
        match byte & self.major_limit {
            x @ 0 ..= 0x17 => Ok(x.into()),
            0x18 => pull_one(self.name, reader).map(Into::into),
            0x19 => {
                let mut buf = [0; 2];
                pull_exact(self.name, reader, &mut buf)?;
                Ok(u16::from_be_bytes(buf))
            },
            _ => Err(Error::mismatch(self.name, byte))
        }
    }

    #[inline]
    fn decode_u32<'de, R: Read<'de>>(self, reader: &mut R) -> Result<u32, Error<R::Error>> {
        let byte = pull_one(self.name, reader)?;
        match byte & self.major_limit {
            x @ 0 ..= 0x17 => Ok(x.into()),
            0x18 => pull_one(self.name, reader).map(Into::into),
            0x19 => {
                let mut buf = [0; 2];
                pull_exact(self.name, reader, &mut buf)?;
                Ok(u16::from_be_bytes(buf).into())
            },
            0x1a => {
                let mut buf = [0; 4];
                pull_exact(self.name, reader, &mut buf)?;
                Ok(u32::from_be_bytes(buf))
            }
            _ => Err(Error::mismatch(self.name, byte))
        }
    }

    #[inline]
    pub(crate) fn decode_u64<'de, R: Read<'de>>(self, reader: &mut R) -> Result<u64, Error<R::Error>> {
        let byte = pull_one(self.name, reader)?;
        match byte & self.major_limit {
            x @ 0 ..= 0x17 => Ok(x.into()),
            0x18 => pull_one(self.name, reader).map(Into::into),
            0x19 => {
                let mut buf = [0; 2];
                pull_exact(self.name, reader, &mut buf)?;
                Ok(u16::from_be_bytes(buf).into())
            },
            0x1a => {
                let mut buf = [0; 4];
                pull_exact(self.name, reader, &mut buf)?;
                Ok(u32::from_be_bytes(buf).into())
            },
            0x1b => {
                let mut buf = [0; 8];
                pull_exact(self.name, reader, &mut buf)?;
                Ok(u64::from_be_bytes(buf))
            },
            _ => Err(Error::mismatch(self.name, byte))
        }
    }
}

macro_rules! decode_ux {
    ( $( $t:ty , $decode_fn:ident );* $( ; )? ) => {
        $(
            impl<'de> Decode<'de> for $t {
                #[inline]
                fn decode<R: Read<'de>>(reader: &mut R) -> Result<Self, Error<R::Error>> {
                    TypeNum::new(&stringify!($t), major::UNSIGNED).$decode_fn(reader)
                }
            }
        )*
    }
}

macro_rules! decode_nx {
    ( $( $t:ty , $decode_fn:ident );* $( ; )? ) => {
        $(
            impl<'de> Decode<'de> for types::Negative<$t> {
                #[inline]
                fn decode<R: Read<'de>>(reader: &mut R) -> Result<Self, Error<R::Error>> {
                    TypeNum::new(&concat!("-", stringify!($t)), major::NEGATIVE)
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
            impl<'de> Decode<'de> for $t {
                #[inline]
                fn decode<R: Read<'de>>(reader: &mut R) -> Result<Self, Error<R::Error>> {
                    let name = &stringify!($t);
                    let unsigned_name = &concat!("+", stringify!($t));
                    let negative_name = &concat!("-", stringify!($t));
                    let byte = peek_one(name, reader)?;

                    match if_major(byte) {
                        major::UNSIGNED => {
                            let v = TypeNum::new(unsigned_name, major::UNSIGNED).$decode_fn(reader)?;
                            <$t>::try_from(v)
                                .map_err(|_| Error::cast_overflow(unsigned_name))
                        },
                        major::NEGATIVE => {
                            let v = TypeNum::new(negative_name, major::NEGATIVE).$decode_fn(reader)?;
                            let v = <$t>::try_from(v)
                                .map_err(|_| Error::cast_overflow(negative_name))?;
                            let v = -v;
                            let v = v.checked_sub(1)
                                .ok_or_else(|| Error::arithmetic_overflow(negative_name, error::ArithmeticOverflow::Underflow))?;
                            Ok(v)
                        },
                        _ => Err(Error::mismatch(name, byte))
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
fn decode_x128<'de, R: Read<'de>>(name: error::StaticStr, reader: &mut R) -> Result<[u8; 16], Error<R::Error>> {
    let len = decode_len(name, major::BYTES, reader)?;
    let len = len.ok_or_else(|| Error::require_length(name, None))?;

    let mut buf = [0; 16];
    if let Some(pos) = buf.len().checked_sub(len) {
        pull_exact(name, reader, &mut buf[pos..])?;
        Ok(buf)
    } else {
        Err(Error::length_overflow(name, len))
    }
}

impl<'de> Decode<'de> for u128 {
    #[inline]
    fn decode<R: Read<'de>>(reader: &mut R) -> Result<Self, Error<R::Error>> {
        let name = &"u128";
        let name_bytes = &"u128::bytes";
        let name_tag = &"u128::tag";

        let byte = peek_one(name, reader)?;
        if if_major(byte) == major::UNSIGNED {
            u64::decode(reader).map(Into::into)
        } else {
            let tag = TypeNum::new(name, major::TAG).decode_u8(reader)?;
            if tag == 2 {
                let buf = decode_x128(name_bytes, reader)?;
                Ok(u128::from_be_bytes(buf))
            } else {
                Err(Error::mismatch(name_tag, tag))
            }
        }
    }
}

impl<'de> Decode<'de> for i128 {
    #[inline]
    fn decode<R: Read<'de>>(reader: &mut R) -> Result<Self, Error<R::Error>> {
        let name = &"i128";
        let name_ubytes = &"+i128::bytes";
        let name_nbytes = &"-i128::bytes";
        let name_tag = &"i128::tag";

        let byte = peek_one(name, reader)?;
        match if_major(byte) {
            major::UNSIGNED => u64::decode(reader).map(Into::into),
            major::NEGATIVE => {
               // Negative numbers can be as big as 2^64, hence decode them as unsigned 64-bit
               // value first.
               let n = TypeNum::new(name, major::NEGATIVE).decode_u64(reader)?;
               let n = i128::from(n);
               let n = -n;
               let n = n - 1;
               Ok(n)
            },
            _ => {
                let tag = TypeNum::new(name, major::TAG).decode_u8(reader)?;
                match tag {
                    2 => {
                        let buf = decode_x128(name_ubytes, reader)?;
                        let n = u128::from_be_bytes(buf);
                        let n = i128::try_from(n).map_err(|_| Error::cast_overflow(name))?;
                        Ok(n)
                    },
                    3 => {
                        let buf = decode_x128(name_nbytes, reader)?;
                        let n = u128::from_be_bytes(buf);
                        let n = i128::try_from(n).map_err(|_| Error::cast_overflow(name))?;
                        let n = -n;
                        let n = n.checked_sub(1)
                            .ok_or_else(|| Error::arithmetic_overflow(name, error::ArithmeticOverflow::Underflow))?;
                        Ok(n)
                    },
                    _ => Err(Error::mismatch(name_tag, tag))
                }
            }
        }
    }
}

#[inline]
fn decode_len<'de, R: Read<'de>>(name: error::StaticStr, major: u8, reader: &mut R)
    -> Result<Option<usize>, Error<R::Error>>
{
    let byte = peek_one(name, reader)?;
    if byte != (marker::START | (major << 5)) {
        let len = TypeNum::new(name, major).decode_u64(reader)?;
        let len = usize::try_from(len).map_err(|_| Error::cast_overflow(name))?;
        Ok(Some(len))
    } else {
        reader.advance(1);
        Ok(None)
    }
}

#[inline]
fn decode_bytes<'de, R: Read<'de>>(name: error::StaticStr, major: u8, reader: &mut R)
    -> Result<&'de [u8], Error<R::Error>>
{
    let len = TypeNum::new(name, major).decode_u64(reader)?;
    let len = usize::try_from(len)
        .map_err(|_| Error::cast_overflow(name))?;

    match reader.fill(len)? {
        Reference::Long(buf) if buf.len() >= len => {
            reader.advance(len);
            Ok(&buf[..len])
        },
        Reference::Long(buf) => Err(Error::require_length(name, Some(buf.len()))),
        Reference::Short(_) => Err(Error::require_borrowed(name))
    }
}

pub struct BytesSequence<'de, R>
where
    R: Read<'de>
{
    name: error::StaticStr,
    major: u8,
    reader: R,
    started: bool,
    level: usize,
    remaining: usize,
    ready: usize,
    _phantom: core::marker::PhantomData<&'de [u8]>
}

pub enum BytesDecodeEvent<'de, 'short> {
    Reserve(usize),
    Bytes(Reference<'de, 'short>),
    End
}

impl<'de, R> BytesSequence<'de, R>
where
    R: Read<'de>
{
    #[inline]
    pub fn next<'short>(&'short mut self) -> Result<BytesDecodeEvent<'de, 'short>, Error<R::Error>> {
        if self.ready != 0 {
            self.reader.advance(self.ready);
            self.ready = 0;
        }

        loop {
            match (self.level, self.remaining) {
                (0, 0) if self.started => return Ok(BytesDecodeEvent::End),
                (0, 0) => {
                    self.started = true;

                    match decode_len(self.name, self.major, &mut self.reader)? {
                        Some(len) => {
                            self.remaining = len;
                            return Ok(BytesDecodeEvent::Reserve(len));
                        },
                        None => {
                            self.level = self.level.checked_add(1)
                                .ok_or_else(|| Error::depth_overflow(self.name))?;
                            continue
                        }
                    }
                },
                (level, 0) => {
                    if is_break(&mut self.reader)? {
                        self.level = level - 1;
                        continue
                    }

                    match decode_len(self.name, self.major, &mut self.reader)? {
                        Some(len) => {
                            self.remaining = len;
                            return Ok(BytesDecodeEvent::Reserve(len));
                        },
                        None => {
                            self.level = self.level.checked_add(1)
                                .ok_or_else(|| Error::depth_overflow(self.name))?;
                            continue
                        }
                    }
                },
                (_, len) => {
                    let buf = self.reader.fill(len)?;

                    if buf.as_ref().is_empty() {
                        return Err(Error::eof(self.name, len));
                    }

                    let buf = buf.take(len);
                    let n = buf.as_ref().len();

                    self.ready = n;
                    self.remaining -= n;

                    return Ok(BytesDecodeEvent::Bytes(buf));
                },
            }
        }
    }
}

impl<'de, R> Drop for BytesSequence<'de, R>
where
    R: Read<'de>
{
    #[inline]
    fn drop(&mut self) {
        if self.ready != 0 {
            self.reader.advance(self.ready);
        }
    }
}

impl<'de, R> BytesSequence<'de, R>
where
    R: Read<'de>
{
    #[inline]
    pub(crate) fn new(name: error::StaticStr, major: u8, reader: R) -> Self {
        BytesSequence {
            name, major, reader,
            started: false,
            level: 0,
            remaining: 0,
            ready: 0,
            _phantom: core::marker::PhantomData
        }
    }
}

#[cfg(feature = "use_alloc")]
#[inline]
fn decode_buf<'de, R>(name: error::StaticStr, major: u8, reader: &mut R)
    -> Result<Vec<u8>, Error<R::Error>>
where
    R: Read<'de>,
{
    let mut output = Vec::new();
    let mut seq = BytesSequence::new(name, major, reader);
    loop {
        match seq.next()? {
            BytesDecodeEvent::Reserve(n) => output.try_reserve(n)
                .map_err(|_| Error::length_overflow(name, n))?,
            BytesDecodeEvent::Bytes(buf) => output.extend_from_slice(buf.as_ref()),
            BytesDecodeEvent::End => break,
        }
    }
    Ok(output)
}

#[cfg(feature = "use_alloc")]
#[inline]
fn decode_cow_buf<'de, R>(name: error::StaticStr, major: u8, reader: &mut R)
    -> Result<crate::alloc::borrow::Cow<'de, [u8]>, Error<R::Error>>
where
    R: Read<'de>,
{
    use crate::alloc::borrow::Cow;

    let mut output = Cow::Borrowed(&[][..]);
    let mut seq = BytesSequence::new(name, major, reader);
    loop {
        match seq.next()? {
            BytesDecodeEvent::Reserve(n) => if let Cow::Owned(output) = &mut output {
                output.try_reserve(n).map_err(|_| Error::length_overflow(name, n))?;
            },
            BytesDecodeEvent::Bytes(buf) => {
                if let Reference::Long(longbuf) = buf {
                    if output.is_empty() {
                        output = Cow::Borrowed(longbuf);
                        continue
                    }
                }

                output.to_mut().extend_from_slice(buf.as_ref());
            },
            BytesDecodeEvent::End => break
        }
    }
    Ok(output)
}

impl<'de> types::Bytes<()> {
    /// Decode into bytes sequence.
    #[inline]
    pub fn decode_into<R, C>(reader: R)
        -> BytesSequence<'de, R>
    where
        R: Read<'de>,
    {
        BytesSequence::new(&"bytes", major::BYTES, reader)
    }
}

impl<'de> Decode<'de> for types::Bytes<&'de [u8]> {
    #[inline]
    fn decode<R: Read<'de>>(reader: &mut R) -> Result<Self, Error<R::Error>> {
        let buf = decode_bytes(&"bytes", major::BYTES, reader)?;
        Ok(types::Bytes(buf))
    }
}

#[cfg(feature = "use_alloc")]
impl<'de> Decode<'de> for types::Bytes<Vec<u8>> {
    #[inline]
    fn decode<R: Read<'de>>(reader: &mut R) -> Result<Self, Error<R::Error>> {
        let buf = decode_buf(&"bytes", major::BYTES, reader)?;
        Ok(types::Bytes(buf))
    }
}

#[cfg(feature = "use_alloc")]
impl<'de> Decode<'de> for types::Bytes<crate::alloc::borrow::Cow<'de, [u8]>> {
    #[inline]
    fn decode<R: Read<'de>>(reader: &mut R) -> Result<Self, Error<R::Error>> {
        let buf = decode_cow_buf(&"bytes", major::BYTES, reader)?;
        Ok(types::Bytes(buf))
    }
}

impl<'de> Decode<'de> for &'de str {
    #[inline]
    fn decode<R: Read<'de>>(reader: &mut R) -> Result<Self, Error<R::Error>> {
        let name = &"str";
        let buf = decode_bytes(name, major::STRING, reader)?;
        core::str::from_utf8(buf).map_err(|_| Error::require_utf8(name))
    }
}

#[cfg(feature = "use_alloc")]
impl<'de> Decode<'de> for String {
    #[inline]
    fn decode<R: Read<'de>>(reader: &mut R) -> Result<Self, Error<R::Error>> {
        let types::BadStr(buf) = <types::BadStr<Vec<u8>>>::decode(reader)?;
        String::from_utf8(buf).map_err(|_| Error::require_utf8(&"str"))
    }
}

#[cfg(feature = "use_alloc")]
impl<'de> Decode<'de> for crate::alloc::borrow::Cow<'de, str> {
    #[inline]
    fn decode<R: Read<'de>>(reader: &mut R) -> Result<Self, Error<R::Error>> {
        use crate::alloc::borrow::Cow;

        let name = &"str";
        let types::BadStr(buf) = <types::BadStr<Cow<'_, [u8]>>>::decode(reader)?;

        match buf {
            Cow::Borrowed(buf) => core::str::from_utf8(buf)
                .map(Cow::Borrowed)
                .map_err(|_| Error::require_utf8(name)),
            Cow::Owned(buf) => String::from_utf8(buf)
                .map(Cow::Owned)
                .map_err(|_| Error::require_utf8(name))
        }
    }
}

impl<'de> types::BadStr<()> {
    /// Decode into string sequence (but no utf8 check).
    #[inline]
    pub fn decode_into<R, C>(reader: R)
        -> BytesSequence<'de, R>
    where
        R: Read<'de>,
    {
        BytesSequence::new(&"str", major::STRING, reader)
    }
}

impl<'de> Decode<'de> for types::BadStr<&'de [u8]> {
    #[inline]
    fn decode<R: Read<'de>>(reader: &mut R) -> Result<Self, Error<R::Error>> {
        let buf = decode_bytes(&"str", major::STRING, reader)?;
        Ok(types::BadStr(buf))
    }
}

#[cfg(feature = "use_alloc")]
impl<'de> Decode<'de> for types::BadStr<Vec<u8>> {
    #[inline]
    fn decode<R: Read<'de>>(reader: &mut R) -> Result<Self, Error<R::Error>> {
        let buf = decode_buf(&"str", major::STRING, reader)?;
        Ok(types::BadStr(buf))
    }
}

#[cfg(feature = "use_alloc")]
impl<'de> Decode<'de> for types::BadStr<crate::alloc::borrow::Cow<'de, [u8]>> {
    #[inline]
    fn decode<R: Read<'de>>(reader: &mut R) -> Result<Self, Error<R::Error>> {
        let buf = decode_cow_buf(&"str", major::STRING, reader)?;
        Ok(types::BadStr(buf))
    }
}

pub struct ArrayStart(pub Option<usize>);

impl<'de> Decode<'de> for ArrayStart {
    #[inline]
    fn decode<R: Read<'de>>(reader: &mut R) -> Result<Self, Error<R::Error>> {
        decode_len(&"array", major::ARRAY, reader).map(ArrayStart)
    }
}

#[cfg(feature = "use_alloc")]
impl<'de, T: Decode<'de>> Decode<'de> for Vec<T> {
    #[inline]
    fn decode<R: Read<'de>>(reader: &mut R) -> Result<Self, Error<R::Error>> {
        let name = &"array";
        let mut arr = Vec::new();

        if !reader.step_in() {
            return Err(Error::depth_overflow(name));
        }
        let mut reader = ScopeGuard(reader, |reader| reader.step_out());
        let reader = &mut *reader;

        if let Some(len) = decode_len(name, major::ARRAY, reader)? {
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

impl<'de> Decode<'de> for MapStart {
    #[inline]
    fn decode<R: Read<'de>>(reader: &mut R) -> Result<Self, Error<R::Error>> {
        decode_len(&"map", major::MAP, reader).map(MapStart)
    }
}

#[cfg(feature = "use_alloc")]
impl<'de, K: Decode<'de>, V: Decode<'de>> Decode<'de> for types::Map<Vec<(K, V)>> {
    #[inline]
    fn decode<R: Read<'de>>(reader: &mut R) -> Result<Self, Error<R::Error>> {
        let name = &"map";
        let mut map = Vec::new();

        if !reader.step_in() {
            return Err(Error::depth_overflow(name));
        }
        let mut reader = ScopeGuard(reader, |reader| reader.step_out());
        let reader = &mut *reader;

        if let Some(len) = decode_len(name, major::MAP, reader)? {
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

impl<'de> Decode<'de> for TagStart {
    #[inline]
    fn decode<R: Read<'de>>(reader: &mut R) -> Result<Self, Error<R::Error>> {
        TypeNum::new(&"tag", major::TAG).decode_u64(reader).map(TagStart)
    }
}

impl<'de, T: Decode<'de>> Decode<'de> for types::Tag<T> {
    #[inline]
    fn decode<R: Read<'de>>(reader: &mut R) -> Result<Self, Error<R::Error>> {
        let tag = TypeNum::new(&"tag", major::TAG).decode_u64(reader)?;
        let value = T::decode(reader)?;
        Ok(types::Tag(tag, value))
    }
}

impl<'de> Decode<'de> for types::Simple {
    #[inline]
    fn decode<R: Read<'de>>(reader: &mut R) -> Result<Self, Error<R::Error>> {
        let n = TypeNum::new(&"simple", major::SIMPLE).decode_u8(reader)?;
        Ok(types::Simple(n))
    }
}

impl<'de> Decode<'de> for bool {
    #[inline]
    fn decode<R: Read<'de>>(reader: &mut R) -> Result<Self, Error<R::Error>> {
        let name = &"bool";
        let byte = peek_one(name, reader)?;
        let ret = match byte {
            marker::FALSE => false,
            marker::TRUE => true,
            _ => return Err(Error::mismatch(name, byte))
        };
        reader.advance(1);
        Ok(ret)
    }
}

impl<'de, T: Decode<'de>> Decode<'de> for Option<T> {
    #[inline]
    fn decode<R: Read<'de>>(reader: &mut R) -> Result<Self, Error<R::Error>> {
        let byte = peek_one(&"option", reader)?;
        if byte != marker::NULL && byte != marker::UNDEFINED {
            T::decode(reader).map(Some)
        } else {
            reader.advance(1);
            Ok(None)
        }
    }
}

impl<'de> Decode<'de> for types::F16 {
    #[inline]
    fn decode<R: Read<'de>>(reader: &mut R) -> Result<Self, Error<R::Error>> {
        let name = &"f16";
        let byte = peek_one(name, reader)?;

        if byte == marker::F16 {
            reader.advance(1);
            let mut buf = [0; 2];
            pull_exact(name, reader, &mut buf)?;
            Ok(types::F16(u16::from_be_bytes(buf)))
        } else {
            Err(Error::mismatch(name, byte))
        }
    }
}

#[cfg(feature = "half-f16")]
impl<'de> Decode<'de> for half::f16 {
    #[inline]
    fn decode<R: Read<'de>>(reader: &mut R) -> Result<Self, Error<R::Error>> {
        let types::F16(n) = types::F16::decode(reader)?;
        Ok(half::f16::from_bits(n))
    }
}

impl<'de> Decode<'de> for f32 {
    #[inline]
    fn decode<R: Read<'de>>(reader: &mut R) -> Result<Self, Error<R::Error>> {
        let name = &"f32";
        let byte = peek_one(name, reader)?;

        if byte == marker::F32 {
            reader.advance(1);
            let mut buf = [0; 4];
            pull_exact(name, reader, &mut buf)?;
            Ok(f32::from_be_bytes(buf))
        } else {
            Err(Error::mismatch(name, byte))
        }
    }
}

impl<'de> Decode<'de> for f64 {
    #[inline]
    fn decode<R: Read<'de>>(reader: &mut R) -> Result<Self, Error<R::Error>> {
        let name = &"f64";
        let byte = peek_one(name, reader)?;

        if byte == marker::F64 {
            reader.advance(1);
            let mut buf = [0; 8];
            pull_exact(name, reader, &mut buf)?;
            Ok(f64::from_be_bytes(buf))
        } else {
            Err(Error::mismatch(name, byte))
        }
    }
}

/// Ignore an arbitrary object
pub struct IgnoredAny;

impl<'de> Decode<'de> for IgnoredAny {
    fn decode<R: Read<'de>>(reader: &mut R) -> Result<Self, Error<R::Error>> {
        let name = &"ignored-any";

        if !reader.step_in() {
            return Err(Error::depth_overflow(name));
        }
        let mut reader = ScopeGuard(reader, |reader| reader.step_out());
        let reader = &mut *reader;

        let byte = peek_one(name, reader)?;

        match if_major(byte) {
            major @ major::UNSIGNED | major @ major::NEGATIVE => {
                let skip = match byte & !(major << 5) {
                    0 ..= 0x17 => 0,
                    0x18 => 1,
                    0x19 => 2,
                    0x1a => 4,
                    0x1b => 8,
                    _ => return Err(Error::mismatch(name, byte))
                };
                skip_exact(reader, skip + 1)?;
            },
            major @ major::BYTES | major @ major::STRING |
            major @ major::ARRAY | major @ major::MAP => {
                if let Some(len) = decode_len(name, major, reader)? {
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
                let _tag = TypeNum::new(&"tag", major).decode_u64(reader)?;
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
                    _ => return Err(Error::unsupported(name, byte))
                };
                skip_exact(reader, skip + 1)?;
            },
            _ => return Err(Error::unsupported(name, byte))
        }

        Ok(IgnoredAny)
    }
}

#[inline]
pub fn is_break<'de, R: Read<'de>>(reader: &mut R) -> Result<bool, Error<R::Error>> {
    if peek_one(&"break", reader)? == marker::BREAK {
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
