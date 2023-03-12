//! TOML related methods.

use std::path::Path;

use cargo_toml::Manifest;
use miette::IntoDiagnostic;
use tokio::fs;

use crate::constants::filename;

/// Reads and parses the `Cargo.toml` at the given project directory.
pub async fn read_cargo_toml<P: AsRef<Path>>(origin: P) -> miette::Result<Manifest> {
    toml::from_str::<Manifest>(
        &fs::read_to_string(origin.as_ref().join(filename::CARGO_TOML))
            .await
            .into_diagnostic()?,
    )
    .into_diagnostic()
}
