//! Signed self-updates from the ZapFast Business GitHub releases.
//!
//! This channel has its own repository, application identity and publisher key.
//! It never accepts ZapFast or FastsApp packages.

pub use fastframe_update::{
    CHECK_INTERVAL, DownloadState, Installation, Kind, Prepared, Release, Source, Unsupported,
    Updater,
};
use fastframe_update::{ReqwestTransport, UpdateConfig};

/// ZapFast Business release identity. No upstream or legacy aliases are trusted.
pub const CONFIG: UpdateConfig = UpdateConfig {
    publisher_key: Some(include_str!("../assets/business-update-public-key.hex")),
    ..UpdateConfig::new(
        "DLangkamer/zapfast-business",
        "ZapFast Business",
        "zapfast-business",
        env!("CARGO_PKG_VERSION"),
    )
};

/// An updater using the same proxy-aware HTTP client as WhatsApp traffic.
pub fn updater() -> anyhow::Result<Updater> {
    let mut builder = reqwest::blocking::Client::builder();
    if let Some(proxy) = crate::proxy::reqwest_proxy() {
        builder = builder.proxy(proxy);
    }
    Ok(Updater::new(CONFIG, ReqwestTransport::new(builder)?))
}

pub const fn enabled() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_config_is_valid_and_business_only() {
        CONFIG.validate().unwrap();
        assert_eq!(CONFIG.current_version, env!("CARGO_PKG_VERSION"));
        assert_eq!(CONFIG.repository, "DLangkamer/zapfast-business");
        assert_eq!(CONFIG.slug, "zapfast-business");
        assert!(CONFIG.publisher_key.is_some());
        assert!(CONFIG.legacy_names.is_empty());
        assert!(CONFIG.legacy_windows_installs.is_empty());
    }

    #[test]
    fn the_updater_starts_on_the_business_github_channel() {
        assert!(enabled());
        assert!(updater().unwrap().source().is_github());
    }
}
