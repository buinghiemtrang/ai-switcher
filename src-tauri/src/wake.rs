//! This independent build never installed the original macOS scheduled-prime daemons.
//! Keep the command contract without inspecting or changing another application's services.

pub fn legacy_daemons_installed() -> bool {
    false
}

pub fn uninstall_legacy_daemons() -> anyhow::Result<()> {
    Ok(())
}
