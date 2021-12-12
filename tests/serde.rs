#![cfg(all(feature = "serde1", feature = "use_std"))]

use std::collections::BTreeMap;
use serde::{ Serialize, Deserialize };
use cbor4ii::core::dec;
use cbor4ii::serde::to_vec;


#[track_caller]
fn de<'a,T>(bytes: &'a [u8], _value: &T)
    -> T
where T: Deserialize<'a>
{
    match cbor4ii::serde::from_slice(bytes) {
        Ok(t) => t,
        Err(err) => panic!("{:?}: {:?}", err, bytes)
    }
}

macro_rules! assert_test {
    ( $value:expr ) => {{
        let buf = to_vec(Vec::new(), &$value).unwrap();
        let value = de(&buf, &$value);
        assert_eq!(value, $value, "{:?}", buf);
    }}
}

#[test]
fn test_serialize_compat() {
    let value = vec![
        Some(0x99u32),
        None,
        Some(0x33u32)
    ];
    assert_test!(value);

    #[derive(Serialize, Deserialize, Eq, PartialEq, Debug)]
    enum Enum {
        Unit,
        NewType(i32),
        Tuple(String, bool),
        Struct { x: i32, y: i32 },
    }
    assert_test!(Enum::Unit);
    assert_test!(Enum::NewType(0x999));
    assert_test!(Enum::Tuple("123".into(), false));
    assert_test!(Enum::Struct { x: 0x99, y: -0x99 });

    #[derive(Serialize, Deserialize, Eq, PartialEq, Debug)]
    #[serde(untagged)]
    enum UntaggedEnum {
        Bar(i32),
        Foo(Box<str>)
    }

    #[derive(Serialize, Deserialize, Eq, PartialEq, Debug)]
    struct NewType<T>(T);

    #[derive(Serialize, Deserialize, Eq, PartialEq, Debug)]
    struct Test<'a> {
        name: char,
        test: TestMap,
        #[serde(with = "serde_bytes")]
        bytes: Vec<u8>,
        #[serde(with = "serde_bytes")]
        bytes2: Vec<u8>,
        map: BTreeMap<String, Enum>,
        untag: (UntaggedEnum, UntaggedEnum),
        new: NewType<UntaggedEnum>,
        some: Option<()>,
        str_ref: &'a str
    }
    #[derive(Serialize, Deserialize, Eq, PartialEq, Debug)]
    struct TestMap(BTreeMap<TestObj, BoxSet>);
    #[derive(Serialize, Deserialize, Eq, PartialEq, Ord, PartialOrd, Debug)]
    struct TestObj(String);
    #[derive(Serialize, Deserialize, Eq, PartialEq, Debug)]
    struct BoxSet(Vec<TestObj>);

    let test = Test {
        name: 'G',
        test: TestMap({
            let mut map = BTreeMap::new();
            map.insert(TestObj("obj".into()), BoxSet(Vec::new()));
            map.insert(TestObj("obj2".into()), BoxSet(vec![
                TestObj("obj3".into()),
                TestObj("obj4".into()),
                TestObj(String::new())
            ]));
            map
        }),
        bytes: Vec::from("bbbbbbbbbbb".as_bytes()),
        bytes2: Vec::new(),
        map: {
            let mut map = BTreeMap::new();
            map.insert("key0".into(), Enum::Unit);
            map.insert("key1".into(), Enum::Tuple("value".into(), true));
            map.insert("key2".into(), Enum::Struct {
                x: -1,
                y: 0x123
            });
            map.insert("key3".into(), Enum::NewType(-999));
            map
        },
        untag: (UntaggedEnum::Foo("a".into()), UntaggedEnum::Bar(0)),
        new: NewType(UntaggedEnum::Foo("???".into())),
        some: Some(()),
        str_ref: "hello world"
    };
    assert_test!(test);

    assert_test!(Some(10u32));

    assert_test!(Some(vec![(10u128, 99999i128)]));
}

#[test]
fn test_serde_enum_flatten() {
    #[derive(Debug, Copy, Clone, Eq, PartialEq, Deserialize, Serialize)]
    pub enum Platform {
        Amd64,
    }

    #[derive(Debug, Eq, PartialEq, Deserialize, Serialize)]
    pub struct Package {
        #[serde(flatten)]
        pub flattened: Flattened,
    }

    #[derive(Debug, Eq, PartialEq, Deserialize, Serialize)]
    pub struct Flattened {
        pub platform: Platform
    }

    let pkg = Package {
        flattened: Flattened {
            platform: Platform::Amd64,
        },
    };
    let pkgs = vec![pkg];

    assert_test!(pkgs);
}

#[test]
#[cfg(feature = "serde1-value")]
fn test_serde_value() {
    use cbor4ii::core::Value;

    assert_test!(Value::Map(vec![(
        Value::Text("a".into()),
        Value::Bool(false)
    )]));

    assert_test!(Value::Integer(u64::MAX as i128 + 99));
}

#[test]
fn test_serde_cow() {
    use std::borrow::Cow;
    use std::convert::Infallible;

    struct SlowReader<'de>(&'de [u8]);

    impl<'de> dec::Read<'de> for SlowReader<'de> {
        type Error = Infallible;

        #[inline]
        fn fill<'b>(&'b mut self, _want: usize) -> Result<dec::Reference<'de, 'b>, Self::Error> {
            Ok(if self.0.is_empty() {
                dec::Reference::Long(self.0)
            } else {
                dec::Reference::Long(&self.0[..1])
            })
        }

        #[inline]
        fn advance(&mut self, n: usize) {
            let len = core::cmp::min(self.0.len(), n);
            self.0 = &self.0[len..];
        }
    }

    pub fn short_from_slice<'a, T>(buf: &'a [u8]) -> Result<T, dec::Error<Infallible>>
    where
        T: serde::Deserialize<'a>,
    {
        let reader = SlowReader(buf);
        let mut deserializer = cbor4ii::serde::de::Deserializer::new(reader);
        serde::Deserialize::deserialize(&mut deserializer)
    }

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    #[serde(untagged)]
    enum CowStr<'a> {
        #[serde(borrow)]
        Borrowed(&'a str),
        Owned(String)
    }

    impl CowStr<'_> {
        fn as_ref(&self) -> &str {
            match self {
                CowStr::Borrowed(v) => v,
                CowStr::Owned(v) => v.as_str()
            }
        }
    }

    assert_test!(Cow::Borrowed("123"));
    assert_test!(CowStr::Borrowed("321"));

    let input = "1234567";
    let buf = to_vec(Vec::new(), &input).unwrap();

    // real cow str ok
    let value: CowStr = cbor4ii::serde::from_slice(&buf).unwrap();
    if let CowStr::Borrowed(s) = value {
        assert_eq!(input, s);
    } else {
        panic!()
    }

    // real cow str
    let value: CowStr = short_from_slice(&buf).unwrap();
    assert_eq!(input, value.as_ref(), "{:?}", buf);

    // owned str
    let value: Cow<str> = short_from_slice(&buf).unwrap();
    assert_eq!(input, value.as_ref(), "{:?}", buf);
}

#[test]
fn test_serde_skip() {
    #[derive(Serialize, Deserialize, Eq, PartialEq, Debug)]
    struct SkipIt {
        a: Option<u8>,
        #[serde(skip_deserializing)]
        b: Option<u8>,
        c: Option<u8>
    }

    let skipit = SkipIt {
        a: Some(0x1),
        b: Some(0xff),
        c: Some(0xfb)
    };
    let buf = to_vec(Vec::new(), &skipit).unwrap();
    let value = de(&buf, &skipit);
    assert_eq!(value.a, skipit.a);
    assert_eq!(value.b, None);
    assert_eq!(value.c, skipit.c);


    #[derive(Serialize, Deserialize, Eq, PartialEq, Debug)]
    struct Full {
        a: u32,
        b: String,
        c: Vec<u64>,
        d: Option<Box<Full>>,
    }

    #[derive(Serialize, Deserialize, Eq, PartialEq, Debug)]
    struct Missing1 {
        a: u32,
        b: String,
        c: Vec<u64>,
    }

    #[derive(Serialize, Deserialize, Eq, PartialEq, Debug)]
    struct Missing2 {
        a: u32,
        b: String,
        d: Option<Box<Full>>
    }

    let input = Full {
        a: u32::MAX,
        b: String::from("short"),
        c: vec![0x1, 0x2, 0x3, u64::MAX],
        d: Some(Box::new(Full {
            a: 0x42,
            b: String::new(),
            c: vec![u32::MAX.into()],
            d: None
        }))
    };
    let buf = to_vec(Vec::new(), &input).unwrap();

    let value: Missing1 = cbor4ii::serde::from_slice(&buf).unwrap();
    assert_eq!(value.a, input.a);
    assert_eq!(value.b, input.b);
    assert_eq!(value.c, input.c);

    let value: Missing2 = cbor4ii::serde::from_slice(&buf).unwrap();
    assert_eq!(value.a, input.a);
    assert_eq!(value.b, input.b);
    assert_eq!(value.d, input.d);
}

#[test]
fn test_serde_any_u128() {
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    #[serde(untagged)]
    enum AnyU128 {
        U128(u128),
        Str(String)
    }

    let input = AnyU128::U128(u64::MAX as u128 + 99);
    let buf = to_vec(Vec::new(), &input).unwrap();

    // serde_cbor bug
    {
        let value: AnyU128 = serde_cbor::from_slice(&buf).unwrap();
        assert_ne!(input, value);
        assert_eq!(value, AnyU128::Str("\u{1}\u{0}\u{0}\u{0}\u{0}\u{0}\u{0}\u{0}b".into()))
    }

    // serde no support https://github.com/serde-rs/serde/issues/1682
    assert!(cbor4ii::serde::from_slice::<AnyU128>(&buf).is_err())
}
