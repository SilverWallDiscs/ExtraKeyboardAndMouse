use anyhow::Result;
use tracing::info;
use tracing_subscriber::EnvFilter;

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    info!("ExtraKeyboardAndMouse starting");
    info!("This is an early project scaffold; input capture is not enabled yet.");

    run_platform_notice();

    Ok(())
}

#[cfg(target_os = "windows")]
fn run_platform_notice() {
    info!("Platform: Windows");
    info!("Next milestone: enumerate physical keyboards and mice with Windows Raw Input.");
}

#[cfg(not(target_os = "windows"))]
fn run_platform_notice() {
    info!("Platform: unsupported");
    info!("The first implementation targets Windows only.");
}
