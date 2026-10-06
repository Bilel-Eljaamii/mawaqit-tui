//! Binary entry point — thin composition root (ADR-0001).

use mawaqit_tui::{runtime, ui::app::AppModel};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    runtime::run(AppModel::new()).await
}
