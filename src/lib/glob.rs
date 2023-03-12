//! Custom `glob` implementation.

use std::path::{Path, PathBuf};

use async_recursion::async_recursion;
use globset::{GlobBuilder, GlobSet, GlobSetBuilder};
use miette::IntoDiagnostic;
use tokio::fs::{self, DirEntry};

/// Custom `glob` implementation to filter through pathnames in a directory.
///
/// Returns all the matching paths based on the given `path` and included/excluded globs.
pub async fn glob<P, I, E>(
    path: P,
    include_globs: I,
    exclude_globs: E,
    literal_seperator: bool,
) -> miette::Result<Vec<PathBuf>>
where
    P: AsRef<Path> + Send + Sync,
    I: IntoIterator<Item = String>,
    E: IntoIterator<Item = String>,
{
    let include_globset = create_globset(include_globs, literal_seperator)?;
    let exclude_globset = create_globset(exclude_globs, literal_seperator)?;

    let mut matches = vec![];
    recursively_read_dir_mut(&path, &mut |entry| {
        let is_match = entry
            .path()
            .strip_prefix(&path)
            .ok()
            .and_then(|relative_path| relative_path.to_str())
            .map(|s| include_globset.is_match(s) && !exclude_globset.is_match(s))
            .unwrap_or(false);

        if is_match {
            matches.push(entry.path());
        }
    })
    .await;

    Ok(matches)
}

/// Create a [`GlobSet`] from the given `globs`.
fn create_globset<G: IntoIterator<Item = String>>(
    globs: G,
    literal_seperator: bool,
) -> miette::Result<GlobSet> {
    let mut globset_builder = GlobSetBuilder::new();
    for glob in globs {
        globset_builder.add(
            GlobBuilder::new(&glob)
                .literal_separator(literal_seperator)
                .build()
                .into_diagnostic()?,
        );
    }

    globset_builder.build().into_diagnostic()
}

/// Recursively read the given directory with mutable borrowed callback on each entry.
#[async_recursion]
async fn recursively_read_dir_mut<P, F>(path: &P, cb: &mut F)
where
    P: AsRef<Path> + Send + Sync,
    F: FnMut(DirEntry) + Send + Sync,
{
    if let Ok(mut read_dir) = fs::read_dir(path).await {
        while let Ok(Some(entry)) = read_dir.next_entry().await {
            if let Ok(metadata) = entry.metadata().await {
                if metadata.is_dir() {
                    recursively_read_dir_mut(&path.as_ref().join(entry.file_name()), cb).await;
                }

                cb(entry);
            }
        }
    }
}
