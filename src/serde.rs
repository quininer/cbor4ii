pub mod ser;

#[cfg(feature = "use_std")]
mod io_writer {
    use std::io;
    use std::collections::TryReserveError;
    use serde::Serialize;
    use super::ser;
    use crate::core::enc;

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

pub use io_writer::{ to_writer, to_vec };
