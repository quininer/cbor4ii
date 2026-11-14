#![doc = include_str!("../README.md")]
#![cfg_attr(not(feature = "use_std"), no_std)]

#[cfg(feature = "use_alloc")]
extern crate alloc;

mod util;
pub mod core;

#[cfg(feature = "serde1_noalloc")]
pub mod serde;
