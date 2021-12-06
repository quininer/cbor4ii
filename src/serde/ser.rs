use core::fmt;
use serde::Serialize;
use crate::core::types;
use crate::core::enc::{ self, Encode };


pub struct Serializer<W> {
    writer: W
}

impl<W> Serializer<W> {
    pub fn new(writer: W) -> Serializer<W> {
        Serializer { writer }
    }
}

impl<'a, W: enc::Write> serde::Serializer for &'a mut Serializer<W> {
    type Ok = ();
    type Error = enc::Error<W::Error>;

    type SerializeSeq = Collect<'a, W>;
    type SerializeTuple = BoundedCollect<'a, W>;
    type SerializeTupleStruct = BoundedCollect<'a, W>;
    type SerializeTupleVariant = BoundedCollect<'a, W>;
    type SerializeMap = Collect<'a, W>;
    type SerializeStruct = BoundedCollect<'a, W>;
    type SerializeStructVariant = BoundedCollect<'a, W>;

    #[inline]
    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        v.encode(&mut self.writer)?;
        Ok(())
    }

    #[inline]
    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        v.encode(&mut self.writer)?;
        Ok(())
    }

    #[inline]
    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        v.encode(&mut self.writer)?;
        Ok(())
    }

    #[inline]
    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        v.encode(&mut self.writer)?;
        Ok(())
    }

    #[inline]
    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        v.encode(&mut self.writer)?;
        Ok(())
    }

    #[inline]
    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        v.encode(&mut self.writer)?;
        Ok(())
    }

    #[inline]
    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        v.encode(&mut self.writer)?;
        Ok(())
    }

    #[inline]
    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        v.encode(&mut self.writer)?;
        Ok(())
    }

    #[inline]
    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        v.encode(&mut self.writer)?;
        Ok(())
    }

    #[inline]
    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        v.encode(&mut self.writer)?;
        Ok(())
    }

    #[inline]
    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        v.encode(&mut self.writer)?;
        Ok(())
    }

    #[inline]
    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        let mut buf = [0; 4];
        self.serialize_str(v.encode_utf8(&mut buf))
    }

    #[inline]
    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        v.encode(&mut self.writer)?;
        Ok(())
    }

    #[inline]
    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        types::Bytes(v).encode(&mut self.writer)?;
        Ok(())
    }

    #[inline]
    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        types::Null.encode(&mut self.writer)?;
        Ok(())
    }

    #[inline]
    fn serialize_some<T: Serialize + ?Sized>(self, value: &T)
        -> Result<Self::Ok, Self::Error>
    {
        value.serialize(self)
    }

    #[inline]
    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        self.serialize_none()
    }

    #[inline]
    fn serialize_unit_struct(self, _name: &'static str)
        -> Result<Self::Ok, Self::Error>
    {
        self.serialize_unit()
    }

    #[inline]
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str
    ) -> Result<Self::Ok, Self::Error> {
        self.serialize_str(variant)
    }

    #[inline]
    fn serialize_newtype_struct<T: Serialize + ?Sized>(
        self,
        name: &'static str,
        value: &T
    ) -> Result<Self::Ok, Self::Error> {
        value.serialize(self)
    }

    #[inline]
    fn serialize_newtype_variant<T: Serialize + ?Sized>(
        self,
        name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        value: &T
    ) -> Result<Self::Ok, Self::Error> {
        enc::MapStartBounded(1).encode(&mut self.writer)?;
        value.serialize(self)
    }

    #[inline]
    fn serialize_seq(self, len: Option<usize>)
        -> Result<Self::SerializeSeq, Self::Error>
    {
        if let Some(len) = len {
            enc::ArrayStartBounded(len).encode(&mut self.writer)?;
        } else {
            enc::ArrayStartUnbounded.encode(&mut self.writer)?;
        }
        Ok(Collect {
            bounded: len.is_some(),
            ser: self
        })
    }

    #[inline]
    fn serialize_tuple(self, len: usize)
        -> Result<Self::SerializeTuple, Self::Error>
    {
        enc::ArrayStartBounded(len).encode(&mut self.writer)?;
        Ok(BoundedCollect { ser: self })
    }

    #[inline]
    fn serialize_tuple_struct(self, _name: &'static str, len: usize)
        -> Result<Self::SerializeTupleStruct, Self::Error>
    {
        self.serialize_tuple(len)
    }

    #[inline]
    fn serialize_tuple_variant(
        self,
        name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        enc::MapStartBounded(1).encode(&mut self.writer)?;
        name.encode(&mut self.writer)?;
        enc::ArrayStartBounded(len).encode(&mut self.writer)?;
        Ok(BoundedCollect { ser: self })
    }

    #[inline]
    fn serialize_map(self, len: Option<usize>)
        -> Result<Self::SerializeMap, Self::Error>
    {
        if let Some(len) = len {
            enc::MapStartBounded(len).encode(&mut self.writer)?;
        } else {
            enc::MapStartUnbounded.encode(&mut self.writer)?;
        }
        Ok(Collect {
            bounded: len.is_some(),
            ser: self
        })
    }

    #[inline]
    fn serialize_struct(self, _name: &'static str, len: usize)
        -> Result<Self::SerializeStruct, Self::Error>
    {
        enc::MapStartBounded(len).encode(&mut self.writer)?;
        Ok(BoundedCollect { ser: self })
    }

    #[inline]
    fn serialize_struct_variant(
        self,
        name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        enc::MapStartBounded(1).encode(&mut self.writer)?;
        name.encode(&mut self.writer)?;
        enc::MapStartBounded(len).encode(&mut self.writer)?;
        Ok(BoundedCollect { ser: self })
    }

    #[inline]
    fn serialize_i128(self, v: i128) -> Result<Self::Ok, Self::Error> {
        v.encode(&mut self.writer)?;
        Ok(())
    }

    #[inline]
    fn serialize_u128(self, v: u128) -> Result<Self::Ok, Self::Error> {
        v.encode(&mut self.writer)?;
        Ok(())
    }

    #[inline]
    fn collect_str<T: ?Sized>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: fmt::Display,
    {
        use core::fmt::Write;

        enc::StrStart.encode(&mut self.writer)?;
        let mut writer = FmtWriter {
            inner: &mut self.writer,
            error: None
        };
        if write!(&mut writer, "{}", value).is_err() {
            return Err(enc::Error::Write(writer.error.unwrap()));
        }
        enc::End.encode(&mut self.writer)?;

        Ok(())
    }

    #[inline]
    fn is_human_readable(&self) -> bool {
        false
    }
}

pub struct Collect<'a, W> {
    bounded: bool,
    ser: &'a mut Serializer<W>
}

pub struct BoundedCollect<'a, W> {
    ser: &'a mut Serializer<W>
}

impl<W: enc::Write> serde::ser::SerializeSeq for Collect<'_, W> {
    type Ok = ();
    type Error = enc::Error<W::Error>;

    #[inline]
    fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T)
        -> Result<(), Self::Error>
    {
        value.serialize(&mut *self.ser)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        if !self.bounded {
            enc::End.encode(&mut self.ser.writer)?;
        }

        Ok(())
    }
}

impl<W: enc::Write> serde::ser::SerializeTuple for BoundedCollect<'_, W> {
    type Ok = ();
    type Error = enc::Error<W::Error>;

    #[inline]
    fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T)
        -> Result<(), Self::Error>
    {
        value.serialize(&mut *self.ser)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<W: enc::Write> serde::ser::SerializeTupleStruct for BoundedCollect<'_, W> {
    type Ok = ();
    type Error = enc::Error<W::Error>;

    #[inline]
    fn serialize_field<T: Serialize + ?Sized>(&mut self, value: &T)
        -> Result<(), Self::Error>
    {
        value.serialize(&mut *self.ser)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<W: enc::Write> serde::ser::SerializeTupleVariant for BoundedCollect<'_, W> {
    type Ok = ();
    type Error = enc::Error<W::Error>;

    #[inline]
    fn serialize_field<T: Serialize + ?Sized>(&mut self, value: &T)
        -> Result<(), Self::Error>
    {
        value.serialize(&mut *self.ser)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<W: enc::Write> serde::ser::SerializeMap for Collect<'_, W> {
    type Ok = ();
    type Error = enc::Error<W::Error>;

    #[inline]
    fn serialize_key<T: Serialize + ?Sized>(&mut self, key: &T)
        -> Result<(), Self::Error>
    {
        key.serialize(&mut *self.ser)
    }

    #[inline]
    fn serialize_value<T: Serialize + ?Sized>(&mut self, value: &T)
        -> Result<(), Self::Error>
    {
        value.serialize(&mut *self.ser)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        if !self.bounded {
            enc::End.encode(&mut self.ser.writer)?;
        }

        Ok(())
    }
}

impl<W: enc::Write> serde::ser::SerializeStruct for BoundedCollect<'_, W> {
    type Ok = ();
    type Error = enc::Error<W::Error>;

    #[inline]
    fn serialize_field<T: Serialize + ?Sized>(&mut self, key: &'static str, value: &T)
        -> Result<(), Self::Error>
    {
        key.serialize(&mut *self.ser)?;
        value.serialize(&mut *self.ser)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<W: enc::Write> serde::ser::SerializeStructVariant for BoundedCollect<'_, W> {
    type Ok = ();
    type Error = enc::Error<W::Error>;

    #[inline]
    fn serialize_field<T: Serialize + ?Sized>(&mut self, key: &'static str, value: &T)
        -> Result<(), Self::Error>
    {
        key.serialize(&mut *self.ser)?;
        value.serialize(&mut *self.ser)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }

    #[inline]
    fn skip_field(&mut self, key: &'static str) -> Result<(), Self::Error> {
        Ok(())
    }
}

struct FmtWriter<'a, W: enc::Write> {
    inner: &'a mut W,
    error: Option<W::Error>
}

impl<W: enc::Write> fmt::Write for FmtWriter<'_, W> {
    #[inline]
    fn write_str(&mut self, input: &str) -> fmt::Result {
        match self.inner.push(input.as_bytes()) {
            Ok(()) => Ok(()),
            Err(err) => {
                self.error = Some(err);
                Err(fmt::Error)
            }
        }
    }
}
