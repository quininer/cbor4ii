//! serde support

mod ser;
#[cfg(feature = "use_alloc")] mod de;

#[cfg(feature = "use_std")]
mod io_writer {
    use std::io;
    use serde::Serialize;
    use crate::core::enc;
    use crate::serde::ser;

    struct IoWrite<W>(W);

    impl<W: io::Write> enc::Write for IoWrite<W> {
        type Error = io::Error;

        #[inline]
        fn push(&mut self, input: &[u8]) -> Result<(), Self::Error> {
            self.0.write_all(input)
        }
    }

    /// Serializes a value to a writer.
    pub fn to_writer<W, T>(writer: &mut W, value: &T)
        -> Result<(), enc::Error<io::Error>>
    where
        W: io::Write,
        T: Serialize
    {
        let writer = IoWrite(writer);
        let mut writer = ser::Serializer::new(writer);
        value.serialize(&mut writer)
    }
}

#[cfg(feature = "use_alloc")]
mod buf_writer {
    use crate::alloc::vec::Vec;
    use crate::alloc::collections::TryReserveError;
    use serde::Serialize;
    use crate::core::{enc, BufWriter};
    use crate::serde::ser;

    /// Serializes a value to a writer.
    pub fn to_vec<T>(buf: Vec<u8>, value: &T)
        -> Result<Vec<u8>, enc::Error<TryReserveError>>
    where T: Serialize
    {
        let writer = BufWriter::new(buf);
        let mut writer = ser::Serializer::new(writer);
        value.serialize(&mut writer)?;
        Ok(writer.into_inner().into_inner())
    }
}

mod slice_reader {
    use core::convert::Infallible;
    use crate::core::dec;
    use crate::serde::de;

    struct SliceReader<'a> {
        buf: &'a [u8],
        limit: usize
    }

    impl<'de> dec::Read<'de> for SliceReader<'de> {
        type Error = Infallible;

        #[inline]
        fn fill<'b>(&'b mut self, want: usize) -> Result<dec::Reference<'de, 'b>, Self::Error> {
            let len = core::cmp::min(self.buf.len(), want);
            Ok(dec::Reference::Long(&self.buf[..len]))
        }

        #[inline]
        fn advance(&mut self, n: usize) {
            let len = core::cmp::min(self.buf.len(), n);
            self.buf = &self.buf[len..];
        }

        #[inline]
        fn step_in(&mut self) -> bool {
            if let Some(limit) = self.limit.checked_sub(1) {
                self.limit = limit;
                true
            } else {
                false
            }
        }

        #[inline]
        fn step_out(&mut self) {
            self.limit += 1;
        }
    }

    /// Decodes a value from a bytes.
    pub fn from_slice<'a, T>(buf: &'a [u8]) -> Result<T, dec::Error<Infallible>>
    where
        T: serde::Deserialize<'a>,
    {
        let reader = SliceReader { buf, limit: 256 };
        let mut deserializer = de::Deserializer::new(reader);
        serde::Deserialize::deserialize(&mut deserializer)
    }
}

#[cfg(feature = "use_std")]
mod io_buf_reader {
    use std::io::{ self, BufRead };
    use crate::core::dec;
    use crate::serde::de;

    struct IoReader<R> {
        reader: R,
        limit: usize
    }

    impl<'de, R: BufRead> dec::Read<'de> for IoReader<R> {
        type Error = io::Error;

        #[inline]
        fn fill<'b>(&'b mut self, _want: usize) -> Result<dec::Reference<'de, 'b>, Self::Error> {
            let buf = self.reader.fill_buf()?;
            Ok(dec::Reference::Short(buf))
        }

        #[inline]
        fn advance(&mut self, n: usize) {
            self.reader.consume(n);
        }

        #[inline]
        fn step_in(&mut self) -> bool {
            if let Some(limit) = self.limit.checked_sub(1) {
                self.limit = limit;
                true
            } else {
                false
            }
        }

        #[inline]
        fn step_out(&mut self) {
            self.limit += 1;
        }
    }

    /// Decodes a value from a reader.
    pub fn from_reader<T, R>(reader: R) -> Result<T, dec::Error<io::Error>>
    where
        T: serde::de::DeserializeOwned,
        R: BufRead
    {
        let reader = IoReader { reader, limit: 256 };
        let mut deserializer = de::Deserializer::new(reader);
        serde::Deserialize::deserialize(&mut deserializer)
    }
}

#[cfg(feature = "use_std")] pub use io_writer::to_writer;
#[cfg(feature = "use_alloc")] pub use buf_writer::to_vec;
#[cfg(feature = "use_std")] pub use io_buf_reader::from_reader;
pub use slice_reader::from_slice;

pub use ser::Serializer;

#[cfg(feature = "use_alloc")]
pub use de::Deserializer;
