#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]

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
#[cfg_attr(docsrs, doc(cfg(feature = "fs")))]
pub use file::*;

/// Defines the behavior of an opaque subscription.
pub trait Subscription {}
