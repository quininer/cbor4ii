#![cfg_attr(not(feature = "use_std"), no_std)]

extern crate alloc;

mod core;

#[cfg(feature = "serde1")]
pub mod serde;
