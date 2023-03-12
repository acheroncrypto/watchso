//! Utilities for commands.

use std::{
    fmt::Display,
    path::Path,
    process::{ExitStatus, Output},
};

use miette::IntoDiagnostic;
use tokio::process::Command;

/// Utility struct for [`Command`].
pub struct WCommand(Command);

impl WCommand {
    /// Create a new [`WCommand`].
    pub fn new<C: AsRef<str>>(cmd: C) -> Self {
        let cmd_words = cmd.as_ref().split_whitespace().collect::<Vec<_>>();
        let mut cmd = Command::new(cmd_words[0]);
        cmd.args(&cmd_words[1..]);

        Self(cmd)
    }

    /// Set the current directory of the command.
    pub fn current_dir<D: AsRef<Path>>(&mut self, dir: D) -> &mut Self {
        self.0.current_dir(dir);
        self
    }

    /// Get the output of the command.
    pub async fn output(&mut self) -> miette::Result<ReadableOutput> {
        self.0
            .output()
            .await
            .into_diagnostic()
            .map(|output| output.into())
    }

    /// Spawn the command.
    ///
    /// Returns the exit status of the command.
    pub async fn spawn(&mut self) -> miette::Result<bool> {
        self.0
            .spawn()
            .into_diagnostic()?
            .wait()
            .await
            .into_diagnostic()
            .map(|status| status.success())
    }

    /// Returns whether the given command is installed.
    pub async fn exists<D: Display>(cmd: D) -> bool {
        Self::new(format!("{cmd} --version"))
            .output()
            .await
            .map(|output| output.status().success())
            .unwrap_or(false)
    }
}

/// Utility struct for [`Output`].
pub struct ReadableOutput(Output);

impl ReadableOutput {
    /// Get the exit status of the output.
    pub fn status(&self) -> ExitStatus {
        self.0.status
    }

    /// Get the UTF-8 converted stderr.
    pub fn stderr(&self) -> &str {
        Self::convert(&self.0.stderr)
    }

    /// Get the UTF-8 converted stdout.
    pub fn stdout(&self) -> &str {
        Self::convert(&self.0.stdout)
    }

    /// Convert the given bytes to UTF-8.
    fn convert(bytes: &[u8]) -> &str {
        std::str::from_utf8(bytes).unwrap_or_default()
    }
}

impl From<Output> for ReadableOutput {
    fn from(output: Output) -> Self {
        Self(output)
    }
}
