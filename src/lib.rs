#![cfg_attr(not(feature = "use_std"), no_std)]

extern crate alloc;

mod error;
pub mod core;

#[cfg(feature = "serde1")]
pub mod serde;

pub use error::EncodeError;
