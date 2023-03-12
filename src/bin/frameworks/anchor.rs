use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use async_trait::async_trait;
use watchso::{
    command::WCommand,
    error::WatchError,
    framework::{Framework, WatchableFramework},
    framework_utils::{get_program_name_path_hashmap, ProjectMap},
};

#[derive(Default)]
pub struct Anchor {
    /// Starting directory path
    origin: Arc<PathBuf>,
    /// Map of program names and paths
    project_map: ProjectMap,
}

impl Anchor {
    pub fn new<P: AsRef<Path>>(origin: P) -> Self {
        Self {
            origin: Arc::new(origin.as_ref().to_path_buf()),
            ..Default::default()
        }
    }
}

/// Default implementation works.
impl WatchableFramework for Anchor {}

#[async_trait]
impl Framework for Anchor {
    fn origin(&self) -> &Path {
        self.origin.as_path()
    }

    async fn check_toolset(&self) -> miette::Result<()> {
        const ANCHOR: &str = "anchor";
        if !WCommand::exists(ANCHOR).await {
            Err(WatchError::CommandNotFound(ANCHOR))?
        }

        Ok(())
    }

    async fn map_program_names(&self) -> miette::Result<()> {
        for (name, path) in get_program_name_path_hashmap(self.origin()).await? {
            self.project_map.set_program_path(name, path).await;
        }

        Ok(())
    }

    async fn get_program_path(&self, path: &Path) -> Option<PathBuf> {
        self.project_map.get_program_path(path).await
    }

    async fn build(&self, program_path: &Path) -> WCommand {
        // Changing the current directory to the program's path makes Anchor build only the
        // modified program in the workspace.
        let mut command = WCommand::new("anchor build");
        command.current_dir(program_path);
        command
    }

    async fn deploy(&self, elf_path: &Path) -> WCommand {
        // Anchor still deploys all of the programs in the workspace even after changing the
        // current dir to the program's dir and it is using program dirname as program name
        // instead of manifest's package name. Thus, we get the program name from the dirname
        // and only deploy the modified program.
        self.get_program_path(elf_path)
            .await
            .as_ref()
            .and_then(|path| path.file_name())
            .and_then(|name| name.to_str())
            .map(|name| WCommand::new(format!("anchor deploy -p {name}")))
            .unwrap_or(WCommand::new("anchor deploy"))
    }
}
