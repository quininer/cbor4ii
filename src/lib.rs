#![cfg_attr(not(feature = "use_std"), no_std)]

extern crate alloc;

pub mod error;
pub mod core;

#[cfg(feature = "serde1")]
pub mod serde;
