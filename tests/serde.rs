#![cfg(all(feature = "serde1", feature = "use_std"))]

use std::collections::BTreeMap;
use serde::{ Serialize, Deserialize };
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
#[cfg(feature = "serde-value")]
fn test_serde_value() {
    use cbor4ii::core::Value;

    assert_test!(Value::Map(vec![(
        Value::Text("a".into()),
        Value::Bool(false)
    )]));
}
