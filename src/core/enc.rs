use core::fmt;
use crate::core::types;


#[non_exhaustive]
pub enum Error<E> {
    #[cfg(feature = "serde1")]
    Any(alloc::string::String),
    Write(E)
}

impl<E> From<E> for Error<E> {
    fn from(err: E) -> Error<E> {
        Error::Write(err)
    }
}

#[cfg(feature = "serde1")]
#[cfg(feature = "use_std")]
impl<E: std::error::Error + 'static> serde::ser::Error for Error<E> {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Error::Any(msg.to_string())
    }
}

#[cfg(feature = "serde1")]
#[cfg(not(feature = "use_std"))]
impl<E: fmt::Display + fmt::Debug> serde::ser::Error for Error<E> {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        use crate::alloc::string::ToString;

        Error::Any(msg.to_string())
    }
}

#[cfg(feature = "use_std")]
impl<E: std::error::Error + 'static> std::error::Error for Error<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            #[cfg(feature = "serde1")]
            Error::Any(_) => None,
            Error::Write(err) => Some(err)
        }
    }
}

impl<E: fmt::Debug> fmt::Debug for Error<E> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            #[cfg(feature = "serde1")]
            Error::Any(msg) => fmt::Debug::fmt(msg, f),
            Error::Write(err) => fmt::Debug::fmt(err, f)
        }
    }
}

impl<E: fmt::Display> fmt::Display for Error<E> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            #[cfg(feature = "serde1")]
            Error::Any(msg) => fmt::Display::fmt(msg, f),
            Error::Write(err) => fmt::Display::fmt(err, f)
        }
    }
}

pub trait Write {
    #[cfg(feature = "use_std")]
    type Error: std::error::Error + 'static;

    #[cfg(not(feature = "use_std"))]
    type Error: fmt::Display + fmt::Debug;

    fn push(&mut self, input: &[u8]) -> Result<(), Self::Error>;
}

pub trait Encode {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<(), Error<W::Error>>;
}

struct TypeNum<V> {
    type_: u8,
    value: V
}

mod tag {
    pub const UNSIGNED: u8 = 0x00;
    pub const NEGATIVE: u8 = 0x20;
    pub const BYTES:    u8 = 0x40;
    pub const STRING:   u8 = 0x60;
    pub const ARRAY:    u8 = 0x80;
    pub const MAP:      u8 = 0xa0;
    pub const SIMPLE:   u8 = 0xe0;
}

impl<V> TypeNum<V> {
    const fn new(type_: u8, value: V) -> TypeNum<V> {
        TypeNum { type_, value }
    }
}

impl Encode for TypeNum<u8> {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<(), Error<W::Error>> {
        match self.value {
            x @ 0x00 ..= 0x17 => writer.push(&[self.type_ | x])?,
            x => writer.push(&[self.type_ | 0x18, x])?
        }
        Ok(())
    }
}

impl Encode for TypeNum<u16> {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<(), Error<W::Error>> {
        match u8::try_from(self.value) {
            Ok(x) => TypeNum::new(self.type_, x).encode(writer)?,
            Err(_) => {
                let [x0, x1] = self.value.to_be_bytes();
                writer.push(&[self.type_ | 0x19, x0, x1])?
            }
        }
        Ok(())
    }
}

impl Encode for TypeNum<u32> {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<(), Error<W::Error>> {
        match u16::try_from(self.value) {
            Ok(x) => TypeNum::new(self.type_, x).encode(writer)?,
            Err(_) =>{
                let [x0, x1, x2, x3] = self.value.to_be_bytes();
                writer.push(&[self.type_ | 0x1a, x0, x1, x2, x3])?;
            }
        }
        Ok(())
    }
}

impl Encode for TypeNum<u64> {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<(), Error<W::Error>> {
        match u32::try_from(self.value) {
            Ok(x) => TypeNum::new(self.type_, x).encode(writer)?,
            Err(_) => {
                let [x0, x1, x2, x3, x4, x5, x6, x7] = self.value.to_be_bytes();
                writer.push(&[self.type_ | 0x1b, x0, x1, x2, x3, x4, x5, x6, x7])?;
            }
        }
        Ok(())
    }
}

macro_rules! encode_ux {
    ( $( $t:ty ),* ) => {
        $(
            impl Encode for $t {
                fn encode<W: Write>(&self, writer: &mut W) -> Result<(), Error<W::Error>> {
                    TypeNum::new(tag::UNSIGNED, *self).encode(writer)
                }
            }
        )*
    }
}

macro_rules! encode_nx {
    ( $( $t:ty ),* ) => {
        $(
            impl Encode for types::Negative<$t> {
                fn encode<W: Write>(&self, writer: &mut W) -> Result<(), Error<W::Error>> {
                    TypeNum::new(tag::NEGATIVE, self.0).encode(writer)
                }
            }
        )*
    }
}

macro_rules! encode_ix {
    ( $( $t:ty = $t2:ty );* ) => {
        $(
            impl Encode for $t {
                fn encode<W: Write>(&self, writer: &mut W) -> Result<(), Error<W::Error>> {
                    let x = *self;
                    match <$t2>::try_from(x) {
                        Ok(x) => x.encode(writer),
                        Err(_) => types::Negative((-1 - x) as $t2).encode(writer)
                    }
                }
            }
        )*
    }
}

encode_ux!(u8, u16, u32, u64);
encode_nx!(u8, u16, u32, u64);
encode_ix!(
    i8 = u8;
    i16 = u16;
    i32 = u32;
    i64 = u64
);

impl Encode for types::Bytes<&'_ [u8]> {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<(), Error<W::Error>> {
        TypeNum::new(tag::BYTES, self.0.len() as u64).encode(writer)?;
        writer.push(self.0)?;
        Ok(())
    }
}

pub struct BytesStart;

impl Encode for BytesStart {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<(), Error<W::Error>> {
        writer.push(&[0x5f])?;
        Ok(())
    }
}

impl Encode for &'_ str {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<(), Error<W::Error>> {
        TypeNum::new(tag::STRING, self.len() as u64).encode(writer)?;
        writer.push(self.as_bytes())?;
        Ok(())
    }
}

impl Encode for types::BadStr<&'_ [u8]> {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<(), Error<W::Error>> {
        TypeNum::new(tag::STRING, self.0.len() as u64).encode(writer)?;
        writer.push(self.0)?;
        Ok(())
    }
}

#[cfg(feature = "bstr")]
impl Encode for &'_ bstr::BStr {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<(), Error<W::Error>> {
        types::BadStr(self.as_ref()).encode(writer)
    }
}

pub struct StrStart;

impl Encode for StrStart {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<(), Error<W::Error>> {
        writer.push(&[0x7f])?;
        Ok(())
    }
}

impl<T: Encode> Encode for &'_ [T] {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<(), Error<W::Error>> {
        ArrayStartBounded(self.len()).encode(writer)?;
        for value in self.iter() {
            value.encode(writer)?;
        }
        Ok(())
    }
}

pub struct ArrayStartBounded(pub usize);
pub struct ArrayStartUnbounded;

impl Encode for ArrayStartBounded {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<(), Error<W::Error>> {
        TypeNum::new(tag::ARRAY, self.0 as u64).encode(writer)?;
        Ok(())
    }
}

impl Encode for ArrayStartUnbounded {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<(), Error<W::Error>> {
        writer.push(&[0x9f])?;
        Ok(())
    }
}

pub struct Map<'a, K, V>(&'a [(K, V)]);

impl<K: Encode, V: Encode> Encode for Map<'_, K, V> {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<(), Error<W::Error>> {
        MapStartBounded(self.0.len()).encode(writer)?;
        for (k, v) in self.0.iter() {
            k.encode(writer)?;
            v.encode(writer)?;
        }
        Ok(())
    }
}

pub struct MapStartBounded(pub usize);
pub struct MapStartUnbounded;

impl Encode for MapStartBounded {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<(), Error<W::Error>> {
        TypeNum::new(tag::MAP, self.0 as u64).encode(writer)?;
        Ok(())
    }
}

impl Encode for MapStartUnbounded {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<(), Error<W::Error>> {
        writer.push(&[0xbf])?;
        Ok(())
    }
}

impl<T: Encode> Encode for types::Tag<T> {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<(), Error<W::Error>> {
        writer.push(&[self.0])?;
        self.1.encode(writer)
    }
}

impl Encode for types::Simple {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<(), Error<W::Error>> {
        TypeNum::new(tag::SIMPLE, self.0).encode(writer)
    }
}

impl Encode for bool {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<(), Error<W::Error>> {
        writer.push(&[if *self {
            0xf5
        } else {
            0xf4
        }])?;
        Ok(())
    }
}

pub struct Null;

impl Encode for Null {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<(), Error<W::Error>> {
        writer.push(&[0xf6])?;
        Ok(())
    }
}

impl Encode for types::Undefined {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<(), Error<W::Error>> {
        writer.push(&[0xf7])?;
        Ok(())
    }
}

#[cfg(feature = "half-f16")]
impl Encode for half::f16 {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<(), Error<W::Error>> {
        let [x0, x1] = self.to_be_bytes();
        writer.push(&[0xf9, x0, x1])?;
        Ok(())
    }
}

impl Encode for types::F16 {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<(), Error<W::Error>> {
        let [x0, x1] = self.0.to_be_bytes();
        writer.push(&[0xf9, x0, x1])?;
        Ok(())
    }
}

impl Encode for f32 {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<(), Error<W::Error>> {
        let [x0, x1, x2, x3] = self.to_be_bytes();
        writer.push(&[0xfa, x0, x1, x2, x3])?;
        Ok(())
    }
}

impl Encode for f64 {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<(), Error<W::Error>> {
        let [x0, x1, x2, x3, x4, x5, x6, x7] = self.to_be_bytes();
        writer.push(&[0xfb, x0, x1, x2, x3, x4, x5, x6, x7])?;
        Ok(())
    }
}

pub struct End;

impl Encode for End {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<(), Error<W::Error>> {
        writer.push(&[0xff])?;
        Ok(())
    }
}

// from https://www.rfc-editor.org/rfc/rfc8949.html#name-examples-of-encoded-cbor-da
#[test]
#[cfg(feature = "use_std")]
fn test_encoded() -> anyhow::Result<()> {
    pub struct Buffer(Vec<u8>);

    impl Write for Buffer {
        type Error = std::convert::Infallible;

        fn push(&mut self, input: &[u8]) -> Result<(), Self::Error> {
            self.0.extend_from_slice(input);
            Ok(())
        }
    }

    fn hex(input: &[u8]) -> String {
        let mut buf = String::from("0x");
        data_encoding::HEXLOWER.encode_append(input, &mut buf);
        buf
    }

    let mut buf = Buffer(Vec::new());

    macro_rules! test {
        ( $( $input:expr , $expected:expr );* $( ; )? ) => {
            $(
                {
                    buf.0.clear();
                    ($input).encode(&mut buf)?;
                    let output = hex(&buf.0);
                    assert_eq!(output, $expected, "{:?}", stringify!($input));
                }
            )*
        }
    }

    let strbuf_ud800_udd51 = {
        let iter = char::decode_utf16([0xd800u16, 0xdd51u16]);
        let mut buf = String::new();
        for ret in iter {
            buf.push(ret?);
        }
        buf
    };

    test!{
        0u64, "0x00";
        1u64, "0x01";
        10u64, "0x0a";
        23u64, "0x17";
        24u64, "0x1818";
        25u64, "0x1819";
        100u64, "0x1864";
        1000u64, "0x1903e8";
        1000000u64, "0x1a000f4240";
        1000000000000u64, "0x1b000000e8d4a51000";
        18446744073709551615u64, "0x1bffffffffffffffff";

        // TODO bignum
        // 18446744073709551616, "0xc249010000000000000000";

        // TODO u64 overflow
        types::Negative((-18446744073709551616i128 - 1) as u64), "0x3bffffffffffffffff";

        // TODO bignum
        // -18446744073709551617, "0xc349010000000000000000";

        -1i64, "0x20";
        -10i64, "0x29";
        -100i64, "0x3863";
        -1000i64, "0x3903e7";

        half::f16::from_f32(0.0), "0xf90000";
        half::f16::from_f32(-0.0), "0xf98000";
        half::f16::from_f32(1.0), "0xf93c00";
        1.1f64, "0xfb3ff199999999999a";
        half::f16::from_f32(1.5), "0xf93e00";
        half::f16::from_f32(65504.0), "0xf97bff";
        100000.0f32, "0xfa47c35000";
        3.4028234663852886e+38f32, "0xfa7f7fffff";
        1.0e+300f64, "0xfb7e37e43c8800759c";
        half::f16::from_f32(5.960464477539063e-8), "0xf90001";
        half::f16::from_f32(0.00006103515625), "0xf90400";
        half::f16::from_f32(-4.0), "0xf9c400";
        -4.1f64, "0xfbc010666666666666";
        half::f16::INFINITY, "0xf97c00";
        half::f16::NAN, "0xf97e00";
        half::f16::NEG_INFINITY, "0xf9fc00";
        f32::INFINITY, "0xfa7f800000";
        f32::NAN, "0xfa7fc00000";
        f32::NEG_INFINITY, "0xfaff800000";
        f64::INFINITY, "0xfb7ff0000000000000";
        f64::NAN, "0xfb7ff8000000000000";
        f64::NEG_INFINITY, "0xfbfff0000000000000";

        false, "0xf4";
        true, "0xf5";
        Null, "0xf6";
        types::Undefined, "0xf7";

        types::Simple(16), "0xf0";
        types::Simple(255), "0xf8ff";

        // TODO tag

        types::Bytes(&[0u8; 0][..]), "0x40";
        types::Bytes(&[0x01, 0x02, 0x03, 0x04][..]), "0x4401020304";

        "", "0x60";
        "a", "0x6161";
        "IETF", "0x6449455446";
        "\"\\", "0x62225c";
        "\u{00fc}", "0x62c3bc";
        "\u{6c34}", "0x63e6b0b4";
        strbuf_ud800_udd51.as_str(), "0x64f0908591";

        &[0u8; 0][..], "0x80";
        &[1u8, 2, 3][..], "0x83010203";

        // TODO any array

        &[1u8, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25][..],
        "0x98190102030405060708090a0b0c0d0e0f101112131415161718181819";

        Map(&[(0u8, 0u8); 0]), "0xa0";
        Map(&[(1u8, 2u8), (3u8, 4u8)]), "0xa201020304";

        // TODO any map

        Map(&[("a", "A"), ("b", "B"), ("c", "C"), ("d", "D"), ("e", "E")]),
        "0xa56161614161626142616361436164614461656145";

        // TODO more map and array
    }

    Ok(())
}
