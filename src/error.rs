use core::fmt;
use core::num::TryFromIntError;


#[derive(Debug)]
#[non_exhaustive]
pub enum EncodeError<E> {
    #[cfg(feature = "serde1")]
    Msg(alloc::string::String),
    Write(E)
}

impl<E> From<E> for EncodeError<E> {
    fn from(err: E) -> EncodeError<E> {
        EncodeError::Write(err)
    }
}

#[cfg(feature = "serde1")]
#[cfg(feature = "use_std")]
impl<E: std::error::Error + 'static> serde::ser::Error for EncodeError<E> {
    #[cold]
    fn custom<T: fmt::Display>(msg: T) -> Self {
        EncodeError::Msg(msg.to_string())
    }
}

#[cfg(feature = "serde1")]
#[cfg(not(feature = "use_std"))]
impl<E: fmt::Display + fmt::Debug> serde::ser::Error for EncodeError<E> {
    #[cold]
    fn custom<T: fmt::Display>(msg: T) -> Self {
        use crate::alloc::string::ToString;

        EncodeError::Msg(msg.to_string())
    }
}

#[cfg(feature = "use_std")]
impl<E: std::error::Error + 'static> std::error::Error for EncodeError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            #[cfg(feature = "serde1")]
            EncodeError::Msg(_) => None,
            EncodeError::Write(err) => Some(err)
        }
    }
}

impl<E: fmt::Debug> fmt::Display for EncodeError<E> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}


#[derive(Debug)]
#[non_exhaustive]
pub enum DecodeError<E> {
    #[cfg(feature = "serde1")]
    Msg(alloc::string::String),
    Read(E),
    Eof,
    Mismatch {
        expect_major: u8,
        byte: u8
    },
    TypeMismatch {
        name: &'static str,
        byte: u8
    },
    CastOverflow(TryFromIntError),
    Overflow {
        name: &'static str
    },
    RequireBorrowed {
        name: &'static str
    },
    RequireLength {
        name: &'static str,
        expect: usize,
        value: usize
    },
    InvalidUtf8(core::str::Utf8Error)
}

impl<E> DecodeError<E> {
    pub const fn mismatch(major_limit: u8, byte: u8) -> Self {
        DecodeError::Mismatch {
            expect_major: (!major_limit) >> 5,
            byte
        }
    }
}

impl<E> From<E> for DecodeError<E> {
    fn from(err: E) -> DecodeError<E> {
        DecodeError::Read(err)
    }
}

#[cfg(feature = "serde1")]
#[cfg(feature = "use_std")]
impl<E: std::error::Error + 'static> serde::de::Error for DecodeError<E> {
    #[cold]
    fn custom<T: fmt::Display>(msg: T) -> Self {
        DecodeError::Msg(msg.to_string())
    }
}

#[cfg(feature = "serde1")]
#[cfg(not(feature = "use_std"))]
impl<E: fmt::Display + fmt::Debug> serde::de::Error for DecodeError<E> {
    #[cold]
    fn custom<T: fmt::Display>(msg: T) -> Self {
        use crate::alloc::string::ToString;

        DecodeError::Msg(msg.to_string())
    }
}

#[cfg(feature = "use_std")]
impl<E: std::error::Error + 'static> std::error::Error for DecodeError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            #[cfg(feature = "serde1")]
            DecodeError::Msg(_) => None,
            DecodeError::Read(err) => Some(err),
            _ => None
        }
    }
}

impl<E: fmt::Debug> fmt::Display for DecodeError<E> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}
