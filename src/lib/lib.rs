//! Watch [Solana](https://solana.com) programs.
//!
//! # Binary
//! The binary implements hot reloading for popular Solana frameworks.
//!
//! Check out the [repository](https://github.com/acheroncrypto/watchso)
//! for installation and usage of the binary.

#![warn(missing_docs)]

pub mod action;
pub mod command;
pub mod constants;
pub mod error;
pub mod framework;
pub mod framework_utils;
pub mod glob;
pub mod progress;
pub mod toml;

mod watch;
pub use watch::watch;
