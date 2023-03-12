mod frameworks;

use std::env;

use frameworks::get_framework_from_path;
use miette::IntoDiagnostic;
use watchso::watch;

#[tokio::main]
async fn main() -> miette::Result<()> {
    let origin = env::current_dir().into_diagnostic()?;
    let framework = get_framework_from_path(origin).await?;
    watch(framework).await
}
