//! Framework traits.

use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::Arc,
};

use async_trait::async_trait;
use miette::IntoDiagnostic;
use tokio::fs;
use watchexec::filter::Filterer;

use crate::{
    action::WAction,
    command::WCommand,
    constants::{dirname, extension},
    framework_utils::{
        create_globset_filterer, find_and_update_program_id, get_program_path, get_watch_pathset,
        start_test_validator,
    },
    progress::Progress,
};

/// Watchable Solana program framework.
///
/// This trait is a supertrait of [`Framework`].
#[async_trait]
pub trait WatchableFramework: Framework + Send + Sync {
    /// Paths to watch.
    ///
    /// Watchexec puts OS-level watchers on all files under the given paths and it filters them
    /// later with the specified [`WatchableFramework::filterer`] afterwards. This means it's not
    /// a good idea to watch directories with great number of files inside it.
    /// See [watchexec#241](https://github.com/watchexec/watchexec/issues/241) for more information.
    ///
    /// Default implementation is for Rust.
    async fn pathset(&self) -> miette::Result<Vec<PathBuf>> {
        get_watch_pathset(self.origin()).await
    }

    /// Filterer implementation that filters events.
    ///
    /// Default implementation is for Rust.
    async fn filterer(&self) -> Arc<dyn Filterer> {
        let filters = [];
        let ignores = [];
        let extensions = [
            extension::RS,
            extension::TOML,
            extension::SO,
            extension::JSON,
        ];

        create_globset_filterer(self.origin(), &filters, &ignores, &extensions).await
    }

    /// Callback to run when an event has occured and it passed the [`Filterer`].
    ///
    /// Default implementation is for Rust.
    async fn on_action(&self, action: WAction) -> miette::Result<()> {
        // Saving unique program paths because multiple files can be modified within the same
        // action. This way, we don't rebuild the same program in the same action.
        let mut unique_program_paths = HashSet::new();
        for action_path in action.get_unique_paths() {
            if let Some(ext) = action_path.extension().and_then(|ext| ext.to_str()) {
                match ext {
                    extension::RS | extension::TOML => {
                        let program_path = get_program_path(action_path).await?;
                        unique_program_paths.insert(program_path);
                    }
                    extension::SO => {
                        self.deploy(action_path).await.spawn().await?;
                    }
                    extension::JSON => {
                        self.update_program_id(action_path).await?;
                    }
                    _ => (),
                }
            }
        }

        for program_path in unique_program_paths {
            self.build(&program_path).await.spawn().await?;
        }

        Ok(())
    }
}

/// Solana program framework.
#[async_trait]
pub trait Framework: Send + Sync {
    /// Origin is the root directory of the project and other paths will be derived from this path.
    fn origin(&self) -> &Path;

    /// Handle the necessary checks and initialize the framework.
    ///
    /// This is called before watching starts.
    async fn initialize(&self) -> miette::Result<()> {
        self.check_toolset().await?;
        self.map_program_names().await?;

        Progress::new()
            .message("Starting Solana test validator...")
            .success_message("Running Solana test validator")
            .error_message("Could not start Solana test validator")
            .spinner_with(|| async { start_test_validator(self.origin()).await })
            .await?;

        // If `target/deploy` doesn't exist, build the programs first to create the program keypair
        // and program ELF
        let deploy_path = self.origin().join(dirname::TARGET).join(dirname::DEPLOY);
        if !deploy_path.exists() {
            Progress::new()
                .message("Setting up...")
                .success_message("Setup success")
                .error_message("Setup error")
                .spinner_with(|| async { self.build(self.origin()).await.output().await })
                .await?;
        }

        let mut deploy_dir = fs::read_dir(deploy_path).await.into_diagnostic()?;
        let mut keypair_paths = vec![];
        let mut elf_paths = vec![];
        while let Some(entry) = deploy_dir.next_entry().await.into_diagnostic()? {
            if let Some(Some(ext)) = entry.path().extension().map(|ext| ext.to_str()) {
                match ext {
                    extension::JSON => keypair_paths.push(entry.path()),
                    extension::SO => elf_paths.push(entry.path()),
                    _ => (),
                }
            }
        }

        // Get unique build paths
        let mut unique_build_paths = HashSet::new();
        for paths in [&keypair_paths, &elf_paths] {
            for path in paths {
                if let Some(program_path) = self.get_program_path(path).await {
                    unique_build_paths.insert(program_path);
                }
            }
        }

        Progress::new()
            .message("Checking program ids...")
            .success_message("Program ids are up to date")
            .error_message("Couldn't update program ids")
            .progress_with(keypair_paths, |keypair_path| async move {
                self.update_program_id(&keypair_path).await
            })
            .await?;

        Progress::new()
            .message("Building...")
            .success_message("Built programs")
            .error_message("Couldn't build programs")
            .progress_with(unique_build_paths, |build_path| async move {
                self.build(&build_path).await.output().await
            })
            .await?;

        Progress::new()
            .message("Deploying programs...")
            .success_message("Deployed programs")
            .error_message("Couldn't deploy programs")
            .progress_with(elf_paths, |elf_path| async move {
                self.deploy(&elf_path).await.output().await
            })
            .await?;

        println!();

        Ok(())
    }

    /// Check the installed toolsets, e.g Solana CLI.
    async fn check_toolset(&self) -> miette::Result<()>;

    /// Read and cache the program names with their paths to not use filesystem on every action.
    async fn map_program_names(&self) -> miette::Result<()>;

    /// Get the program's root directory path based on the given path.
    ///
    /// The given path can be any path that allows a way to find the program's path, e.g program's
    /// keypair file is named after the program's name and it can be used to get the program's path.
    async fn get_program_path(&self, path: &Path) -> Option<PathBuf>;

    /// Update the program id.
    ///
    /// Default implementation is for Rust.
    async fn update_program_id(&self, program_keypair_path: &Path) -> miette::Result<()> {
        if let Some(program_path) = self.get_program_path(program_keypair_path).await {
            find_and_update_program_id(program_path, program_keypair_path).await?;
        }

        Ok(())
    }

    /// Build command to run.
    async fn build(&self, program_path: &Path) -> WCommand;

    /// Deploy command to run.
    async fn deploy(&self, elf_path: &Path) -> WCommand;
}
