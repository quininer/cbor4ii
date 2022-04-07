use core::marker::PhantomData;
use crate::core::{ enc, dec };


pub struct RawValue<'de>(u8, &'de [u8]);

struct RawValueReader<'r, 'de, R>
where R: dec::Read<'de>
{
    reader: &'r mut R,
    readn: usize,
    _phantom: PhantomData<&'de [u8]>
}

impl<'r, 'de, R> RawValueReader<'r, 'de, R>
where R: dec::Read<'de>
{
    #[inline]
    fn new(reader: &'r mut R) -> RawValueReader<'r, 'de, R> {
        RawValueReader {
            reader,
            readn: 0,
            _phantom: PhantomData
        }
    }
}

impl<'r, 'de, R> dec::Read<'de> for RawValueReader<'r, 'de, R>
where R: dec::Read<'de>
{
    type Error = R::Error;

    #[inline]
    fn fill<'short>(&'short mut self, want: usize) -> Result<dec::Reference<'de, 'short>, Self::Error> {
        let buf = match self.reader.fill(self.readn + want)? {
            dec::Reference::Long(buf)
                if buf.len() >= self.readn => dec::Reference::Long(&buf[self.readn..]),
            dec::Reference::Long(_) => dec::Reference::Long(&[]),
            dec::Reference::Short(buf) => dec::Reference::Short(buf)
        };

        Ok(buf)
    }

    #[inline]
    fn advance(&mut self, n: usize) {
        self.readn += n;
    }

    #[inline]
    fn step_in(&mut self) -> bool {
        self.reader.step_in()
    }

    #[inline]
    fn step_out(&mut self) {
        self.reader.step_out()
    }
}

impl<'r, 'de, R> Drop for RawValueReader<'r, 'de, R>
where R: dec::Read<'de>
{
    #[inline]
    fn drop(&mut self) {
        self.reader.advance(self.readn);
    }
}

impl<'de> dec::Decode<'de> for RawValue<'de> {
    #[inline]
    fn decode_with<R: dec::Read<'de>>(byte: u8, reader: &mut R) -> Result<Self, dec::Error<R::Error>> {
        let mut reader = RawValueReader::new(reader);
        let _ignore = dec::IgnoredAny::decode_with(byte, &mut reader)?;

        let buf = match reader.reader.fill(reader.readn).map_err(dec::Error::Read)? {
            dec::Reference::Long(buf)
                if buf.len() >= reader.readn => &buf[..reader.readn],
            dec::Reference::Long(buf) => return Err(dec::Error::RequireLength {
                name: "raw-value",
                expect: reader.readn,
                value: buf.len()
            }),
            dec::Reference::Short(_) => return Err(dec::Error::RequireBorrowed {
                name: "raw-value"
            })
        };

        Ok(RawValue(byte, buf))
    }
}

impl<'de> enc::Encode for RawValue<'de> {
    #[inline]
    fn encode<W: enc::Write>(&self, writer: &mut W) -> Result<(), enc::Error<W::Error>> {
        writer.push(&[self.0]).map_err(enc::Error::Write)?;
        writer.push(self.1).map_err(enc::Error::Write)
    }
}
