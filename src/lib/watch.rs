//! Custom watch implementation with [`watchexec`].

use std::{sync::Arc, time::Duration};

use console::style;
use miette::IntoDiagnostic;
use watchexec::{
    action::{Action, Outcome},
    config::{InitConfig, RuntimeConfig},
    Watchexec,
};

use crate::{action::WAction, error::WatchError, framework::WatchableFramework};

/// Watch the changes based on the specific [`WatchableFramework`] implementation.
pub async fn watch(framework: Arc<dyn WatchableFramework>) -> miette::Result<()> {
    framework.initialize().await?;

    let mut runtime = RuntimeConfig::default();

    runtime
        .pathset(framework.pathset().await?)
        .filterer(framework.filterer().await)
        .action_throttle(Duration::from_millis(200))
        .on_action(move |action| {
            let framework = framework.clone();
            async move { on_action(action, framework).await }
        });

    let init = InitConfig::default();

    let watchexec = Watchexec::new(init, runtime)?;
    watchexec.main().await.into_diagnostic()??;

    Ok(())
}

/// Top level action handler.
async fn on_action(
    action: Action,
    framework: Arc<dyn WatchableFramework>,
) -> Result<(), WatchError> {
    let action = WAction::new(action);

    if action.is_interrupt() || action.is_terminate() {
        action
            .take()
            .outcome(Outcome::both(Outcome::Stop, Outcome::Exit));
        return Ok(());
    }

    if let Err(err) = framework.on_action(action).await {
        eprintln!("{} {}", style("[ERR]").red().bold(), err);
    }

    Ok(())
}
