use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use async_trait::async_trait;
use tokio::sync::RwLock;
use watchso::{
    command::WCommand,
    framework::{Framework, WatchableFramework},
    framework_utils::{get_bpf_or_sbf, get_program_name_path_hashmap, ProjectMap},
};

#[derive(Default)]
pub struct Native {
    /// Starting directory path
    origin: Arc<PathBuf>,
    /// Map of program names and paths
    project_map: ProjectMap,
    // Full build command to run. Either `cargo build-bpf` or `cargo build-sbf`
    build_cmd: BuildCommand,
}

impl Native {
    pub fn new<P: AsRef<Path>>(origin: P) -> Self {
        Self {
            origin: Arc::new(origin.as_ref().to_path_buf()),
            ..Default::default()
        }
    }
}

/// Default implementation works.
impl WatchableFramework for Native {}

#[async_trait]
impl Framework for Native {
    fn origin(&self) -> &Path {
        self.origin.as_path()
    }

    async fn check_toolset(&self) -> miette::Result<()> {
        let build_cmd = get_bpf_or_sbf().await?;
        self.build_cmd.set(build_cmd).await;

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
        let mut command = WCommand::new(self.build_cmd.get().await);
        command.current_dir(program_path);
        command
    }

    async fn deploy(&self, elf_path: &Path) -> WCommand {
        WCommand::new(format!("solana program deploy {}", elf_path.display()))
    }
}

/// Full build command to run. Using `RwLock` because the process is read heavy.
struct BuildCommand(Arc<RwLock<&'static str>>);

impl BuildCommand {
    /// Get the current build command.
    async fn get(&self) -> &'static str {
        *self.0.read().await
    }

    /// Set the current build command.
    async fn set(&self, build_cmd: &'static str) {
        *self.0.write().await = build_cmd;
    }
}

impl Default for BuildCommand {
    fn default() -> Self {
        Self(Arc::new(RwLock::new("cargo build-sbf")))
    }
}
