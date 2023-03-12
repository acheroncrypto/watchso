//! Progress bars and spinners with consistent behaviour and styles.

use std::future::Future;

use console::Emoji;
use indicatif::{ProgressBar, ProgressStyle};
use tokio::time::Duration;

use crate::constants::emoji;

/// Terminal progress utility struct.
#[derive(Default)]
pub struct Progress<'a> {
    message: Option<&'a str>,
    success_message: Option<&'a str>,
    error_message: Option<&'a str>,
    clear: bool,
}

impl<'a> Progress<'a> {
    /// Create a new [`Progress`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the message that will be displayed while the progress is ongoing.
    pub fn message(&mut self, message: &'a str) -> &mut Self {
        self.message = Some(message);
        self
    }

    /// Set the success message for the progress.
    pub fn success_message(&mut self, message: &'a str) -> &mut Self {
        self.success_message = Some(message);
        self
    }

    /// Set the error message for the progress.
    pub fn error_message(&mut self, message: &'a str) -> &mut Self {
        self.error_message = Some(message);
        self
    }

    /// Set whether the line should be cleared after the progress is finished.
    #[allow(dead_code)]
    pub fn clear(&mut self, clear: bool) -> &mut Self {
        self.clear = clear;
        self
    }

    /// Spawn a spinner with the given callback.
    pub async fn spinner_with<F, R, O>(&self, cb: F) -> miette::Result<O>
    where
        F: Fn() -> R,
        R: Future<Output = miette::Result<O>>,
    {
        let pb = ProgressBar::new_spinner();
        pb.set_style(ProgressStyle::with_template(" {spinner:.green} {msg}").unwrap());
        pb.enable_steady_tick(Duration::from_millis(120));

        if let Some(message) = self.message {
            pb.set_message(message.to_owned());
        }

        let output = cb().await;

        match output {
            Ok(_) => handle_output(&pb, self.success_message, "green", emoji::CHECKMARK),
            Err(_) => handle_output(&pb, self.error_message, "red", emoji::CROSS),
        }

        if self.clear {
            pb.finish_and_clear()
        } else {
            pb.finish()
        }

        output
    }

    /// Spawn a progress bar with the given iterator and run the callback per element.
    pub async fn progress_with<I, T, F, R, O>(&self, iter: I, cb: F) -> miette::Result<()>
    where
        I: IntoIterator<Item = T> + Clone,
        F: Fn(T) -> R,
        R: Future<Output = miette::Result<O>>,
    {
        let vec = iter.into_iter().collect::<Vec<_>>();
        let len = vec.len();
        let width = len.to_string().len();
        let pb = ProgressBar::new(len as u64);
        pb.set_style(
            ProgressStyle::with_template(&format!(
                "[{{pos:>{width}}}/{{len:{width}}}] {{bar:.blue/white}} {{msg}}"
            ))
            .unwrap(),
        );

        if let Some(message) = self.message {
            pb.set_message(message.to_owned());
        }

        for item in vec {
            cb(item).await?;
            pb.inc(1);
        }

        handle_output(&pb, self.success_message, "green", emoji::CHECKMARK);

        if self.clear {
            pb.finish_and_clear()
        } else {
            pb.finish()
        }

        Ok(())
    }
}

/// Show the output message with custom color and emoji prefix after progress has finished.
fn handle_output(pb: &ProgressBar, msg: Option<&str>, color: &str, prefix: Emoji) {
    pb.set_style(
        ProgressStyle::with_template(&format!("{{prefix:.{color}}} {{msg:.{color}}}")).unwrap(),
    );
    pb.set_prefix(format!("{}", prefix));
    if let Some(msg) = msg {
        pb.set_message(msg.to_owned());
    }
}
