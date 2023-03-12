//! Custom watch errors.

use std::path::PathBuf;

use miette::Diagnostic;
use thiserror::Error;

/// Custom error definition for the crate.
#[derive(Error, Diagnostic, Debug)]
pub enum WatchError {
    /// This error occurs when the program runs in a directory that's doesn't contain a Solana program.
    #[error("Invalid program directory: `{0}`")]
    InvalidProgramDirectory(PathBuf),

    /// Command is not installed in user's machine.
    #[error("Command not found: `{0}`")]
    CommandNotFound(&'static str),

    /// This most likely happens when the keypair file is not in a valid form.
    #[error("Could not get keypair file: `{0}`")]
    CouldNotGetKeypair(String),
}
