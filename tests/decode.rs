#![cfg(feature = "use_alloc")]

use std::convert::Infallible;
use cbor4ii::core::Value;
use cbor4ii::core::dec::{ self, Decode };


struct SliceReader<'a> {
    buf: &'a [u8],
    limit: usize
}

impl SliceReader<'_> {
    fn new(buf: &[u8]) -> SliceReader<'_> {
        SliceReader { buf, limit: 256 }
    }
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

#[test]
fn test_decode_value() {
    macro_rules! test {
        ( @ $input:expr ) => {
            let buf = data_encoding::BASE64.decode($input.as_bytes()).unwrap();
            let mut reader = SliceReader::new(buf.as_slice());
            let _ = Value::decode(&mut reader);
        };
        ( $( $input:expr );* $( ; )? ) => {
            $(
                test!(@ $input );
            )*
        }
    }

    test!{
        "ig==";
        "eoY=";
        "v6a/v6a/v7+/pq6urq6urq6urq6urq6urq6urq6urq6urq6urqaurq6urq6urq4krq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6uv7+mv7+/v6aurq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urr+/pr+/v7+mrq6urq6urq6urq6urq6urq6urq6urq6upq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6uQK6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urqSurq6urq6urq6urq6urq6urq6urq6urq6urq6uv7+uJa6urq6urq6urq6urq6urq6urq6urq6urq6urq6uv7+uJa6urq6urq6urq6urq6urq6uQK6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq5Arq6urq6urq6urq6urq6urq6urq6urq6urq5Arq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6upK6urq6urq6urq6urq6urq6urq6urq6urq6urq6/v64lrq6urq6urq6urq6urq6urq6urq6urq6urq6urq6/v64lrq6urq6urq6urq6urq6urq5Arq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urkCurq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6krq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6upq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6upK6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urqaurq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq5Arq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6upK6urq6urq6urq6urq6urq6urq6urq6urq6urq6mrq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq5Arq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6upK6urq6urq6urq6urq6urq6urq6urq6urq6urq6/v64lrq6urq6urq6urq6urq6urq6urq6urq6urq6urq6/v64lrq6urq6urq6urq6urq6urq5Arq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urkCurq6urq6urq6urq6urq6urq6urq6urq6urkCurq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6krq6urq6urq6urq6urq6urq6urq6urq6urq6urr+/riWurq6urq6urq6urq6urq6urq6urq6urq6urq6urr+/riWurq6urq6urq6urq6urq6urkCurq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6uQK6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urqSurq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6mrq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6krq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6upq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urkCurq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6krq6urq6urq6urq6urq6urq6urq6urq6urqSurq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6upK6urq6urq6urq6urq6urq6urq6urq6urq6urq6/v64lrq6urq6urq6urq6urq6urq6urq6urq6urq6urq6/v64lrq6urq6urq6urq6urq6urq5Arq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urkCurq6urq6urq6urq6urq6urq6urq6urq6urkCurq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6krq6urq6urq6urq6urq6urq6urq6urq6urq6urr+/riWurq6urq6urq6urq6urg0AAAAAAAAArq6urq6urr+/riWurq6urq6urq6urq6urq6urkCurq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6uQK6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urqSurq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6mrq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6krq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6upq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urkCurq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6krq6urq6urq6urq6urq6urq6urq6urq6urq6urr+/riWurq6urq6urq6urq6urq6urkCurq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6krq6urq6urq6vrq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urrCwsLCwsLCwsLCwsLCwsLCwsLCwrq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6upq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urkCurq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6/v64lrq6urq6urq6urq6urq6urq5Arq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6upK6urq6urq6ur66urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6wsLCwsLCwsLCwsLCwsLCwsLCwsK6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urqaurq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq5Arq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6upK6urq6urq6urq6urq6urq6urq6urq6urq6urq6/v64lrq6urq6urq6urq6urq6urq5Arq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6upK6urq6urq6ur66urq6urq6urq6urq6urq6urq6/v64lrq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6wsLCwsLCwsLCwsLCwsLCwsLCwsK6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urrCwsLCwsLCwsLCwsLCwsLCwsLCwsIQEAAAAgQCwsLCwsLCwsLCwsK6urq6urq6u";
        "v/b29vYBAAAAAAAABPb29gn29vb29pkAEfb29vb29vb2f3///39//3//f/9//3//f/9//3//f///f/9//3//f/8ICAgICAgI9vf39wgICAgICAgI+EAKCgr39wv1CAgICCgILggItAgICAgICAgICAgAAACgAAgICAAICAgICAgI9vf39wgICAgICAgI+EAKCgr39wv1CAgICCgILggItAgICAgICAgICAgAAACgAAgICA=="
    }
}
