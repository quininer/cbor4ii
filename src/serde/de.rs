use serde::de::{ self, Visitor };
use crate::core::{ major, marker, types };
use crate::core::dec::{ self, Decode };


pub struct Deserializer<R> {
    reader: R
}

impl<R> Deserializer<R> {
    pub fn new(reader: R) -> Deserializer<R> {
        Deserializer { reader }
    }

    pub fn into_inner(self) -> R {
        self.reader
    }
}

macro_rules! deserialize_type {
    ( @ $t:ty , $name:ident , $visit:ident ) => {
        fn $name<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where V: Visitor<'de>
        {
            let value = <$t>::decode(&mut self.reader)?;
            visitor.$visit(value)
        }
    };
    ( $( $t:ty , $name:ident , $visit:ident );* $( ; )? ) => {
        $(
            deserialize_type!(@ $t, $name, $visit);
        )*
    };
}

impl<'de, 'a, R: dec::Read<'de>> serde::Deserializer<'de> for &'a mut Deserializer<R> {
    type Error = dec::Error<R::Error>;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>
    {
        let byte = dec::peek_one(&mut self.reader)?;
        match byte >> 5 {
            major::UNSIGNED => self.deserialize_u64(visitor),
            major::NEGATIVE => self.deserialize_i64(visitor),
            major::BYTES => self.deserialize_byte_buf(visitor),
            major::STRING => self.deserialize_string(visitor),
            major::ARRAY => self.deserialize_seq(visitor),
            major::MAP => self.deserialize_map(visitor),
            _ => match byte {
                marker::FALSE => {
                    self.reader.advance(1);
                    visitor.visit_bool(false)
                },
                marker::TRUE => {
                    self.reader.advance(1);
                    visitor.visit_bool(true)
                },
                marker::NULL | marker::UNDEFINED => {
                    self.reader.advance(1);
                    visitor.visit_none()
                },
                #[cfg(feature = "half-f16")]
                marker::F16 => {
                    self.reader.advance(1);
                    let v = half::f16::decode_with(byte, &mut self.reader)?;
                    visitor.visit_f32(v.into())
                },
                marker::F32 => self.deserialize_f32(visitor),
                marker::F64 => self.deserialize_f32(visitor),
                _ => Err(dec::Error::Unsupported { byte })
            }
        }
    }

    deserialize_type!(
        bool,       deserialize_bool,       visit_bool;

        i8,         deserialize_i8,         visit_i8;
        i16,        deserialize_i16,        visit_i16;
        i32,        deserialize_i32,        visit_i32;
        i64,        deserialize_i64,        visit_i64;

        u8,         deserialize_u8,         visit_u8;
        u16,        deserialize_u16,        visit_u16;
        u32,        deserialize_u32,        visit_u32;
        u64,        deserialize_u64,        visit_u64;

        f32,        deserialize_f32,        visit_f32;
        f64,        deserialize_f64,        visit_f64;

        &str,       deserialize_str,        visit_borrowed_str;
        String,     deserialize_string,     visit_string;
        Vec<u8>,    deserialize_byte_buf,   visit_byte_buf;
    );

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where V: Visitor<'de>
    {
        let sbuf = <&str>::decode(&mut self.reader)?;
        let count = sbuf.chars().count();
        if count == 1 {
            let c = sbuf.chars()
                .next()
                .ok_or(dec::Error::RequireLength {
                    name: "char",
                    expect: 1,
                    value: count
                })?;
            visitor.visit_char(c)
        } else {
            Err(dec::Error::RequireLength {
                name: "char",
                expect: 1,
                value: count
            })
        }
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where V: Visitor<'de>
    {
        let value = <types::Bytes<&[u8]>>::decode(&mut self.reader)?;
        visitor.visit_borrowed_bytes(value.0)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where V: Visitor<'de>
    {
        let byte = dec::peek_one(&mut self.reader)?;
        if byte != marker::NULL && byte != marker::UNDEFINED {
            visitor.visit_some(self)
        } else {
            self.reader.advance(1);
            visitor.visit_none()
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where V: Visitor<'de>
    {
        let byte = dec::pull_one(&mut self.reader)?;
        if byte != marker::NULL && byte != marker::UNDEFINED {
            visitor.visit_unit()
        } else {
            Err(dec::Error::TypeMismatch {
                name: "unit",
                byte
            })
        }
    }

    fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V)
        -> Result<V::Value, Self::Error>
    where V: Visitor<'de>
    {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V)
        -> Result<V::Value, Self::Error>
    where V: Visitor<'de>
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where V: Visitor<'de>
    {
        let seq = Accessor::array(self)?;
        visitor.visit_seq(seq)
    }

    fn deserialize_tuple<V>(
        self,
        len: usize,
        visitor: V
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>
    {
        let seq = Accessor::tuple(self, len)?;
        visitor.visit_seq(seq)
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        len: usize,
        visitor: V
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>
    {
        self.deserialize_tuple(len, visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>
    {
        let map = Accessor::map(self)?;
        visitor.visit_map(map)
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V
    ) -> Result<V::Value, Self::Error>
    where V: Visitor<'de>
    {
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>
    {
        let accessor = Accessor::enum_(self)?;
        visitor.visit_enum(accessor)
    }

    fn deserialize_identifier<V>(self, visitor: V)
        -> Result<V::Value, Self::Error>
    where V: Visitor<'de>
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_ignored_any<V>(self, visitor: V)
        -> Result<V::Value, Self::Error>
    where V: Visitor<'de>
    {
        self.deserialize_any(visitor)
    }

    /*
    fn deserialize_i128<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_u128<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }
    */

    fn is_human_readable(&self) -> bool {
        false
    }
}

struct Accessor<'a, R> {
    de: &'a mut Deserializer<R>,
    len: Option<usize>
}

impl<'de, 'a, R: dec::Read<'de>> Accessor<'a, R> {
    pub fn array(de: &'a mut Deserializer<R>)
        -> Result<Accessor<'a, R>, dec::Error<R::Error>>
    {
        let byte = dec::pull_one(&mut de.reader)?;
        let len = dec::decode_len(major::ARRAY, byte, &mut de.reader)?;
        Ok(Accessor { de, len })
    }

    pub fn tuple(de: &'a mut Deserializer<R>, len: usize)
        -> Result<Accessor<'a, R>, dec::Error<R::Error>>
    {
        let byte = dec::pull_one(&mut de.reader)?;
        let arrlen = dec::decode_len(major::ARRAY, byte, &mut de.reader)?;

        if arrlen == Some(len) {
            Ok(Accessor { de, len: arrlen })
        } else {
            Err(dec::Error::RequireLength {
                name: "tuple",
                expect: len,
                value: arrlen.unwrap_or(0)
            })
        }
    }

    pub fn map(de: &'a mut Deserializer<R>)
        -> Result<Accessor<'a, R>, dec::Error<R::Error>>
    {
        let byte = dec::pull_one(&mut de.reader)?;
        let len = dec::decode_len(major::MAP, byte, &mut de.reader)?;
        Ok(Accessor { de, len })
    }

    pub fn enum_(de: &'a mut Deserializer<R>)
        -> Result<Accessor<'a, R>, dec::Error<R::Error>>
    {
        let byte = dec::pull_one(&mut de.reader)?;
        let len = dec::decode_len(major::MAP, byte, &mut de.reader)?;
        Ok(Accessor { de, len })
    }

}

impl<'de, 'a, R> de::SeqAccess<'de> for Accessor<'a, R>
where
    R: dec::Read<'de>
{
    type Error = dec::Error<R::Error>;

    fn next_element_seed<T>(&mut self, seed: T)
        -> Result<Option<T::Value>, Self::Error>
    where T: de::DeserializeSeed<'de>
    {
        if let Some(len) = self.len.as_mut() {
            if *len > 0 {
                *len -= 1;
                Ok(Some(seed.deserialize(&mut *self.de)?))
            } else {
                Ok(None)
            }
        } else if dec::peek_one(&mut self.de.reader)? != marker::BREAK {
            Ok(Some(seed.deserialize(&mut *self.de)?))
        } else {
            self.de.reader.advance(1);
            Ok(None)
        }
    }

    fn size_hint(&self) -> Option<usize> {
        self.len
    }
}

impl<'de, 'a, R: dec::Read<'de>> de::MapAccess<'de> for Accessor<'a, R> {
    type Error = dec::Error<R::Error>;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where K: de::DeserializeSeed<'de>
    {
        if let Some(len) = self.len.as_mut() {
            if *len > 0 {
                *len -= 1;
                Ok(Some(seed.deserialize(&mut *self.de)?))
            } else {
                Ok(None)
            }
        } else if dec::peek_one(&mut self.de.reader)? != marker::BREAK {
            Ok(Some(seed.deserialize(&mut *self.de)?))
        } else {
            self.de.reader.advance(1);
            Ok(None)
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where V: de::DeserializeSeed<'de>
    {
        seed.deserialize(&mut *self.de)
    }

    fn size_hint(&self) -> Option<usize> {
        self.len
    }
}

impl<'de, 'a, R> de::EnumAccess<'de> for Accessor<'a, R>
where
    R: dec::Read<'de>
{
    type Error = dec::Error<R::Error>;
    type Variant = Accessor<'a, R>;

    fn variant_seed<V>(self, seed: V)
        -> Result<(V::Value, Self::Variant), Self::Error>
    where V: de::DeserializeSeed<'de>
    {
        let variant = seed.deserialize(&mut *self.de)?;
        Ok((variant, self))
    }
}

impl<'de, 'a, R> de::VariantAccess<'de> for Accessor<'a, R>
where
    R: dec::Read<'de>
{
    type Error = dec::Error<R::Error>;

    fn unit_variant(self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error>
    where T: de::DeserializeSeed<'de>
    {
        seed.deserialize(&mut *self.de)
    }

    fn tuple_variant<V>(self, len: usize, visitor: V)
        -> Result<V::Value, Self::Error>
    where V: Visitor<'de>
    {
        use serde::Deserializer;

        self.de.deserialize_tuple(len, visitor)
    }

    fn struct_variant<V>(
        self,
        _fields: &'static [&'static str],
        visitor: V
    ) -> Result<V::Value, Self::Error>
    where V: Visitor<'de>
    {
        use serde::Deserializer;

        self.de.deserialize_map(visitor)
    }
}
