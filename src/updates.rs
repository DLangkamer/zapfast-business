//! Updates are disabled until Business has its own reviewed, signed channel.
//! Retain the upstream interface for small future merges, but never construct
//! a transport, download a release, or run an installation helper.

use fastframe_update::UpdateConfig;
pub use fastframe_update::{
    CHECK_INTERVAL, DownloadState, Installation, Kind, Prepared, Release, Source, Unsupported,
    Updater,
};

/// Reserved Business identity, without upstream aliases or signing keys.
pub const CONFIG: UpdateConfig = UpdateConfig::new(
    "DLangkamer/zapfast-business",
    "ZapFast Business",
    "zapfast-business",
    env!("CARGO_PKG_VERSION"),
);

/// Fail closed, even if a saved setting or a command requests an update.
pub fn updater() -> anyhow::Result<Updater> {
    anyhow::bail!(
        "Updates are disabled in ZapFast Business; install a reviewed Business build manually"
    )
}

/// No independent signed release channel has been configured.
pub fn enabled() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_config_is_valid() {
        CONFIG.validate().unwrap();
        assert_eq!(CONFIG.current_version, env!("CARGO_PKG_VERSION"));
        assert_eq!(CONFIG.slug, "zapfast-business");
    }

    #[test]
    fn the_updater_is_disabled_before_network_or_installation() {
        assert!(updater().is_err());
        assert!(!enabled());
        assert!(CONFIG.legacy_names.is_empty());
    }
}
