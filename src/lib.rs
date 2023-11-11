#![doc = include_str!("../README.md")]

mod composite;
mod default;
mod global;
mod never;
mod shared;
mod single;
mod token;

pub use composite::*;
pub use default::*;
pub use global::*;
pub use never::*;
pub use shared::*;
pub use single::*;
pub use token::*;

#[cfg(feature = "fs")]
mod file;

#[cfg(feature = "fs")]
pub use file::*;
