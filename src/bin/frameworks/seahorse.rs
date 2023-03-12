use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use async_trait::async_trait;
use lazy_static::lazy_static;
use regex::{Regex, RegexBuilder};
use watchexec::filter::Filterer;
use watchso::{
    action::WAction,
    command::WCommand,
    constants::{dirname, extension},
    error::WatchError,
    framework::{Framework, WatchableFramework},
    framework_utils::{
        create_globset_filterer, get_pubkey_from_keypair_path, update_file_program_id_with,
        ProjectMap,
    },
    glob::glob,
};

#[derive(Default)]
pub struct Seahorse {
    /// Starting directory path
    origin: Arc<PathBuf>,
    /// Map of program names and paths
    project_map: ProjectMap,
}

impl Seahorse {
    pub fn new<P: AsRef<Path>>(origin: P) -> Self {
        Self {
            origin: Arc::new(origin.as_ref().to_path_buf()),
            ..Default::default()
        }
    }
}

#[async_trait]
impl WatchableFramework for Seahorse {
    async fn pathset(&self) -> miette::Result<Vec<PathBuf>> {
        let paths = vec![
            Path::new(dirname::TARGET).join(dirname::DEPLOY),
            PathBuf::from(dirname::PROGRAMS_PY),
        ];

        Ok(paths)
    }

    async fn filterer(&self) -> Arc<dyn Filterer> {
        let filters = [];
        let ignores = [];
        let extensions = [extension::PY, extension::SO, extension::JSON];

        create_globset_filterer(self.origin(), &filters, &ignores, &extensions).await
    }

    async fn on_action(&self, action: WAction) -> miette::Result<()> {
        for action_path in action.get_unique_paths() {
            if let Some(ext) = action_path.extension().and_then(|ext| ext.to_str()) {
                match ext {
                    extension::PY => {
                        self.build(action_path).await.spawn().await?;
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

        Ok(())
    }
}

#[async_trait]
impl Framework for Seahorse {
    fn origin(&self) -> &Path {
        self.origin.as_path()
    }

    async fn check_toolset(&self) -> miette::Result<()> {
        const SEAHORSE: &str = "seahorse";
        if !WCommand::exists(SEAHORSE).await {
            Err(WatchError::CommandNotFound(SEAHORSE))?
        }

        Ok(())
    }

    async fn map_program_names(&self) -> miette::Result<()> {
        let paths = glob(
            self.origin().join(dirname::PROGRAMS_PY),
            [format!("*.{}", extension::PY)],
            [],
            true,
        )
        .await?;

        for path in paths {
            if let Some(program_name) = get_program_name_from_path(&path) {
                self.project_map
                    .set_program_path(program_name.to_owned(), path)
                    .await;
            }
        }

        Ok(())
    }

    async fn get_program_path(&self, path: &Path) -> Option<PathBuf> {
        self.project_map.get_program_path(path).await
    }

    async fn update_program_id(&self, program_keypair_path: &Path) -> miette::Result<()> {
        if let Some(program_path) = self.get_program_path(program_keypair_path).await {
            let program_id = get_pubkey_from_keypair_path(program_keypair_path).await?;
            update_seahorse_program_id(program_path, program_id).await?;
        }

        Ok(())
    }

    async fn build(&self, program_path: &Path) -> WCommand {
        match get_program_name_from_path(program_path) {
            Some(program_name) => WCommand::new(format!("seahorse build -p {program_name}")),
            None => WCommand::new("seahorse build"),
        }
    }

    async fn deploy(&self, elf_path: &Path) -> WCommand {
        self.get_program_path(elf_path)
            .await
            .as_ref()
            .and_then(|path| path.file_name())
            .and_then(|name| name.to_str())
            .map(|name| WCommand::new(format!("anchor deploy -p {name}")))
            .unwrap_or(WCommand::new("anchor deploy"))
    }
}

/// Get program name from program's path.
///
/// Seahorse generates Anchor programs based on program's Python file name.
fn get_program_name_from_path(path: &Path) -> Option<&str> {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.trim_end_matches(".py"))
}

/// Update the file at the given path's `declare_id` function with the given program id.
///
/// Returns whether the program id was updated successfully.
async fn update_seahorse_program_id<P, S>(path: P, program_id: S) -> miette::Result<bool>
where
    P: AsRef<Path>,
    S: AsRef<str>,
{
    lazy_static! {
        static ref REGEX: Regex = RegexBuilder::new(r#"^declare_id\(("|')(\w*)("|')\)"#)
            .multi_line(true)
            .build()
            .unwrap();
    };

    update_file_program_id_with(path, &program_id, |content| {
        REGEX.captures(content).and_then(|captures| captures.get(2))
    })
    .await
}
