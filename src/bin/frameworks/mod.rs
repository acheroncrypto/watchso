mod anchor;
mod native;
mod seahorse;

use std::{path::Path, sync::Arc};

use miette::IntoDiagnostic;
use tokio::fs;
use watchso::{
    constants::{dirname, filename},
    error::WatchError,
    framework::WatchableFramework,
};

use self::{anchor::Anchor, native::Native, seahorse::Seahorse};

/// Get a [`WatchableFramework`] from the given path.
///
/// Returns [WatchError::InvalidProgramDirectory] error if the given path is not a valid Solana
/// program directory.
pub async fn get_framework_from_path<P: AsRef<Path>>(
    origin: P,
) -> miette::Result<Arc<dyn WatchableFramework>> {
    let mut item_names = vec![];

    let mut dir = fs::read_dir(&origin).await.into_diagnostic()?;
    while let Some(entry) = dir.next_entry().await.into_diagnostic()? {
        item_names.push(entry.file_name());
    }

    let item_names = item_names
        .iter()
        .filter_map(|item| item.to_str())
        .collect::<Vec<_>>();

    if item_names.contains(&dirname::PROGRAMS_PY) {
        return Ok(Arc::new(Seahorse::new(origin)));
    }
    if item_names.contains(&filename::ANCHOR_TOML) {
        return Ok(Arc::new(Anchor::new(origin)));
    }
    if item_names.contains(&filename::CARGO_TOML) {
        return Ok(Arc::new(Native::new(origin)));
    }

    Err(WatchError::InvalidProgramDirectory(origin.as_ref().into()))?
}
