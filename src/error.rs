use core::fmt;


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

impl<E: fmt::Debug> fmt::Debug for EncodeError<E> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            #[cfg(feature = "serde1")]
            EncodeError::Msg(msg) => fmt::Debug::fmt(msg, f),
            EncodeError::Write(err) => fmt::Debug::fmt(err, f)
        }
    }
}

impl<E: fmt::Display> fmt::Display for EncodeError<E> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            #[cfg(feature = "serde1")]
            EncodeError::Msg(msg) => fmt::Display::fmt(msg, f),
            EncodeError::Write(err) => fmt::Display::fmt(err, f)
        }
    }
}
