use core::fmt;
use crate::core::dec;


#[derive(Debug)]
pub enum DecodeError<E> {
    Core(dec::Error<E>),
    Custom(Box<str>)
}

impl<E> From<dec::Error<E>> for DecodeError<E> {
    #[inline]
    #[cold]
    fn from(err: dec::Error<E>) -> DecodeError<E> {
        DecodeError::Core(err)
    }
}

impl<E> From<E> for DecodeError<E> {
    #[inline]
    #[cold]
    fn from(err: E) -> DecodeError<E> {
        DecodeError::Core(dec::Error::Read(err))
    }
}

#[cfg(feature = "serde1")]
#[cfg(feature = "use_std")]
impl<E: std::error::Error + 'static> serde::de::Error for DecodeError<E> {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        DecodeError::Custom(msg.to_string().into_boxed_str())
    }
}

#[cfg(feature = "serde1")]
#[cfg(not(feature = "use_std"))]
impl<E: fmt::Debug> serde::de::Error for DecodeError<E> {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        #[cfg(not(feature = "use_std"))]
        use crate::alloc::string::ToString;

        DecodeError::Custom(msg.to_string())
    }
}

impl<E: fmt::Debug> fmt::Display for DecodeError<E> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

#[cfg(feature = "serde1")]
#[cfg(not(feature = "use_std"))]
impl<E: fmt::Debug> serde::ser::StdError for DecodeError<E> {}

#[cfg(feature = "use_std")]
impl<E: std::error::Error + 'static> std::error::Error for DecodeError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            DecodeError::Core(err) => Some(err),
            _ => None
        }
    }
}
