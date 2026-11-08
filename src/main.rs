use anyhow::Result;
use tracing::info;
use tracing_subscriber::EnvFilter;

#[cfg(target_os = "windows")]
mod platform;

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    info!("ExtraKeyboardAndMouse v0.1.0");
    info!("Passive physical-device input monitor");
    info!("Normal Windows input is not blocked or changed.");

    #[cfg(target_os = "windows")]
    platform::windows::run()?;

    #[cfg(not(target_os = "windows"))]
    {
        anyhow::bail!("ExtraKeyboardAndMouse currently supports Windows only.");
    }

    Ok(())
}
