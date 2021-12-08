#![cfg_attr(not(feature = "use_std"), no_std)]
#![feature(backtrace)]

#[cfg(feature = "use_alloc")]
extern crate alloc;

mod error;
pub mod core;

#[cfg(feature = "serde1")]
pub mod serde;

pub use error::EncodeError;
