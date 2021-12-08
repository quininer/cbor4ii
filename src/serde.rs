pub mod ser;
pub mod de;

#[cfg(feature = "use_std")]
mod io_writer {
    use std::io;
    use std::collections::TryReserveError;
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

    struct BufWriter(Vec<u8>);

    impl enc::Write for BufWriter {
        type Error = TryReserveError;

        #[inline]
        fn push(&mut self, input: &[u8]) -> Result<(), Self::Error> {
            self.0.try_reserve(input.len())?;
            self.0.extend_from_slice(input);
            Ok(())
        }
    }

    pub fn to_vec<T>(buf: Vec<u8>, value: &T)
        -> Result<Vec<u8>, enc::Error<TryReserveError>>
    where T: Serialize
    {
        let writer = BufWriter(buf);
        let mut writer = ser::Serializer::new(writer);
        value.serialize(&mut writer)?;
        Ok(writer.into_inner().0)
    }
}

mod slice_reader {
    use core::convert::Infallible;
    use crate::core::dec;
    use crate::serde::de;

    struct SliceReader<'a>(&'a [u8]);

    impl<'de> dec::Read<'de> for SliceReader<'de> {
        type Error = Infallible;

        fn fill<'b>(&'b mut self, want: usize) -> Result<dec::Reference<'de, 'b>, Self::Error> {
            let len = core::cmp::min(self.0.len(), want);
            Ok(dec::Reference::Long(&self.0[..len]))
        }

        fn advance(&mut self, n: usize) {
            let len = core::cmp::min(self.0.len(), n);
            self.0 = &self.0[len..];
        }
    }

    pub fn from_slice<'a, T>(buf: &'a [u8]) -> Result<T, dec::Error<Infallible>>
    where
        T: serde::Deserialize<'a>,
    {
        let reader = SliceReader(buf);
        let mut deserializer = de::Deserializer::new(reader);
        let value = serde::Deserialize::deserialize(&mut deserializer)?;
        Ok(value)
    }
}

pub use io_writer::{ to_writer, to_vec };
pub use slice_reader::from_slice;
