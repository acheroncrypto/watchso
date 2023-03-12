//! Utilities for framework implementations.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use lazy_static::lazy_static;
use miette::IntoDiagnostic;
use regex::{Match, Regex, RegexBuilder};
use tokio::{fs, sync::RwLock, time};
use watchexec_filterer_globset::GlobsetFilterer;

use crate::{
    command::WCommand,
    constants::{dirname, extension, filename},
    error::WatchError,
    glob::glob,
    toml::read_cargo_toml,
};

/// A mapping of program names and their paths. Using `RwLock` because the process is read heavy.
#[derive(Default)]
pub struct ProjectMap(Arc<RwLock<HashMap<String, PathBuf>>>);

impl ProjectMap {
    /// Get the program's path from the given path. Mainly used for getting the program path from
    /// program keypair or ELF path.
    pub async fn get_program_path<P: AsRef<Path>>(&self, path: P) -> Option<PathBuf> {
        let program_name = match path.as_ref().extension().map(|ext| ext.to_str()) {
            Some(Some(ext)) => match ext {
                extension::JSON => ProgramName::from_keypair_path(path),
                extension::SO => ProgramName::from_elf_path(path),
                _ => None,
            },
            _ => None,
        };

        match program_name {
            Some(program_name) => match self
                .get_program_path_from_name(program_name.original())
                .await
            {
                Some(program_path) => Some(program_path),
                None => {
                    self.get_program_path_from_name(program_name.kebab_case())
                        .await
                }
            },
            None => None,
        }
    }

    /// Set the program path based on program name.
    pub async fn set_program_path<S, P>(&self, name: S, path: P)
    where
        S: Into<String>,
        P: Into<PathBuf>,
    {
        let mut program_hm = self.0.write().await;
        program_hm.insert(name.into(), path.into());
    }

    /// Get the program path from the program name.
    async fn get_program_path_from_name<S: AsRef<str>>(&self, name: S) -> Option<PathBuf> {
        self.0
            .read()
            .await
            .get(name.as_ref())
            .map(|path| path.to_owned())
    }
}

/// Utility struct to get the program name.
///
/// Solana build tools generate the keypair name as `<program_name>-keypair.json` and ELF name as
/// `<program_name>.so`. Since `<program_name>` is always in snake case, we are not able to get the
/// program name. That's  because the programs named "hello-world" and "hello_world" will have the
/// exact same output files.
#[derive(Debug)]
pub struct ProgramName(String);

impl ProgramName {
    /// Create a new [`ProgramName`] from the given `name`.
    pub fn new<S: Into<String>>(name: S) -> Self {
        Self(name.into())
    }

    /// Get the program's name from the program keypair path.
    ///
    /// This function utilizes the fact that the program keypair names are in the format of
    /// `<program_name>-keypair.json`.
    pub fn from_keypair_path<P: AsRef<Path>>(program_keypair_path: P) -> Option<Self> {
        Self::from_path(program_keypair_path, "-keypair.json")
    }

    /// Get the program's name from the program ELF path.
    ///
    /// This function utilizes the fact that the program ELF names are in the format of
    /// `<program_name>.so`.
    pub fn from_elf_path<P: AsRef<Path>>(program_elf_path: P) -> Option<Self> {
        Self::from_path(program_elf_path, ".so")
    }

    /// Get the program's name by getting the file name from the `path` and stripping the `suffix`.
    fn from_path<P, S>(path: P, suffix: S) -> Option<Self>
    where
        P: AsRef<Path>,
        S: AsRef<str>,
    {
        path.as_ref()
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|name| name.ends_with(suffix.as_ref()))
            .map(|name| Self::new(name.trim_end_matches(suffix.as_ref())))
    }

    /// Reference to the original program name.
    pub fn original(&self) -> &str {
        &self.0
    }

    /// Convert the original program name to kebab-case.
    pub fn kebab_case(&self) -> String {
        self.0.replace('_', "-")
    }
}

/// Start a new test validator by running `solana-test-validator` command.
///
/// This won't have any effect if there is already a running test validator.
///
/// NOTE: This function will spawn a tokio task because `solana-test-validator` command never
/// resolves. It will then sleep for a small duration to give time for the initialization. This
/// means it will not confirm that the test validator has started.
pub async fn start_test_validator<P: Into<PathBuf>>(origin: P) -> miette::Result<()> {
    let origin = origin.into();
    tokio::spawn(async {
        let _ = WCommand::new("solana-test-validator")
            .current_dir(origin)
            .output()
            .await;
    });

    // Wait 2 seconds for the test validator to start
    time::sleep(time::Duration::from_secs(2)).await;

    Ok(())
}

/// Get all the directory paths that will be watched by default.
///
/// If the `origin` is a workspace, the paths will be filtered by `workspace.members` and
/// `workspace.exclude`. Otherwise it's the `src` dir by default.
///
/// Paths always include `target/deploy`.
pub async fn get_watch_pathset<P: AsRef<Path>>(origin: P) -> miette::Result<Vec<PathBuf>> {
    let mut paths = vec![Path::new(dirname::TARGET).join(dirname::DEPLOY)];
    match filter_workspace_programs(origin).await? {
        Some(filtered_paths) => paths.extend(filtered_paths),
        None => paths.push(PathBuf::from(dirname::SRC)),
    }

    Ok(paths)
}

/// Filter workspace programs based on the manifest file at `origin`.
///
/// Returns `Ok(None)` if the `origin` is not a workspace but has a manifest file.
async fn filter_workspace_programs<P: AsRef<Path>>(
    origin: P,
) -> miette::Result<Option<Vec<PathBuf>>> {
    let manifest = read_cargo_toml(&origin).await?;
    match manifest.workspace {
        Some(workspace) => {
            let paths = glob(origin.as_ref(), workspace.members, workspace.exclude, true).await?;
            Ok(Some(paths))
        }
        None => Ok(None),
    }
}

/// Get a mapping of program names and paths based on the manifest file at `origin`.
pub async fn get_program_name_path_hashmap<P: AsRef<Path>>(
    origin: P,
) -> miette::Result<HashMap<String, PathBuf>> {
    let mut program_name_path_hm = HashMap::new();
    let program_paths = filter_workspace_programs(&origin)
        .await?
        .unwrap_or(vec![origin.as_ref().to_path_buf()]);
    for program_path in program_paths {
        if let Ok(manifest) = read_cargo_toml(&program_path).await {
            if let Some(package) = manifest.package {
                program_name_path_hm.insert(package.name, program_path);
            }
        }
    }

    Ok(program_name_path_hm)
}

/// Get program's root path by running `cargo locate-project` command.
pub async fn get_program_path<P: AsRef<Path>>(modified_file_path: P) -> miette::Result<PathBuf> {
    let output = WCommand::new("cargo locate-project --message-format plain")
        .current_dir(modified_file_path.as_ref().parent().unwrap())
        .output()
        .await?;
    if output.status().success() {
        Ok(Path::new(output.stdout().trim_end_matches('\n'))
            .parent()
            .unwrap()
            .to_path_buf())
    } else {
        Err(WatchError::CommandNotFound("cargo locate-project"))?
    }
}

/// Get the keypair's address by running `solana address` command.
pub async fn get_pubkey_from_keypair_path<P: AsRef<Path>>(
    keypair_path: P,
) -> miette::Result<String> {
    let keypair_output = WCommand::new(format!(
        "solana address -k {}",
        keypair_path.as_ref().display()
    ))
    .output()
    .await?;

    if !keypair_output.status().success() {
        return Err(WatchError::CouldNotGetKeypair(
            keypair_output.stderr().into(),
        ))?;
    }

    let program_id = keypair_output.stdout().trim_end_matches('\n');

    Ok(program_id.to_owned())
}

/// Find the file that includes `declare_id!` macro and update the program id if it has changed.
///
/// This function will check `lib.rs` first and **only** if it doesn't find the declaration it will
/// then check all the remaining source files.
pub async fn find_and_update_program_id<P1, P2>(
    program_path: P1,
    program_keypair_path: P2,
) -> miette::Result<()>
where
    P1: AsRef<Path>,
    P2: AsRef<Path>,
{
    // Get the keypair program id
    let program_id = get_pubkey_from_keypair_path(program_keypair_path).await?;

    // Check lib.rs first for the program id
    let src_path = program_path.as_ref().join(dirname::SRC);
    let lib_rs_path = src_path.join(filename::LIB_RS);

    if update_rust_program_id(lib_rs_path, &program_id).await? {
        return Ok(());
    }

    // Check all the other files if the program_id doesn't exist in lib.rs
    let rust_src_paths = glob(src_path, [format!("*.{}", extension::RS)], [], false).await?;
    for path in rust_src_paths {
        if update_rust_program_id(path, &program_id).await? {
            // Not necessary to continue the loop after program id update
            break;
        }
    }

    Ok(())
}

/// Update the file at the given path's `declare_id!` macro with the given program id.
///
/// Returns whether the program id was updated successfully.
async fn update_rust_program_id<P, S>(path: P, program_id: S) -> miette::Result<bool>
where
    P: AsRef<Path>,
    S: AsRef<str>,
{
    lazy_static! {
        static ref REGEX: Regex = RegexBuilder::new(r#"^(([\w]+::)*)declare_id!\("(\w*)"\)"#)
            .multi_line(true)
            .build()
            .unwrap();
    };

    update_file_program_id_with(path, &program_id, |content| {
        REGEX.captures(content).and_then(|captures| captures.get(3))
    })
    .await
}

/// Update the file's `declare_id!` macro with the program id based on the given callback.
///
/// Returns whether the program id was updated successfully.
pub async fn update_file_program_id_with<P, S, F>(
    path: P,
    program_id: S,
    cb: F,
) -> miette::Result<bool>
where
    P: AsRef<Path>,
    S: AsRef<str>,
    F: Fn(&str) -> Option<Match<'_>>,
{
    let mut content = fs::read_to_string(&path).await.into_diagnostic()?;
    if let Some(program_id_match) =
        cb(&content).filter(|program_id_match| program_id_match.as_str() != program_id.as_ref())
    {
        // Update the program id
        content.replace_range(program_id_match.range(), program_id.as_ref());

        // Save the file
        fs::write(&path, content).await.into_diagnostic()?;

        return Ok(true);
    }

    Ok(false)
}

/// Get Solana build tool.
///
/// Checks for `cargo build-sbf` and `cargo build-bpf` in order.
///
/// Returns an error if the Solana build tools are not installed.
pub async fn get_bpf_or_sbf() -> miette::Result<&'static str> {
    const BUILD_SBF: &str = "cargo build-sbf";
    const BUILD_BPF: &str = "cargo build-bpf";

    let build_cmd = if WCommand::exists(BUILD_SBF).await {
        BUILD_SBF
    } else if WCommand::exists(BUILD_BPF).await {
        BUILD_BPF
    } else {
        return Err(WatchError::CommandNotFound("solana"))?;
    };

    Ok(build_cmd)
}

/// Create a globset filterer that will be used to filter the watched files.
///
/// The filterer will always ignore `target`, `test-ledger` and `node_modules` paths.
pub async fn create_globset_filterer<P: AsRef<Path>>(
    origin: P,
    filters: &[&str],
    ignores: &[&str],
    extensions: &[&str],
) -> Arc<GlobsetFilterer> {
    let filters = filters
        .iter()
        .map(|glob| (glob.to_string(), None))
        .collect::<Vec<(String, Option<PathBuf>)>>();
    let ignores = [
        &[
            "**/*/target/**/*",
            "**/*/test-ledger/**/*",
            "**/*/node_modules/**/*",
        ],
        ignores,
    ]
    .concat()
    .iter()
    .map(|glob| (glob.to_string(), None))
    .collect::<Vec<(String, Option<PathBuf>)>>();
    let ignore_files = [];
    let extensions = extensions.iter().map(|ext| ext.into());

    Arc::new(
        GlobsetFilterer::new(origin, filters, ignores, ignore_files, extensions)
            .await
            .unwrap(),
    )
}
