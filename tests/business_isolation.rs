//! Synthetic checks only: never open installed profiles or real credentials.
use keyring_core::api::CredentialStoreApi;
use zapfast::{identity, paths::AppDirs, single_instance};

#[test]
fn separate_profiles_can_hold_instance_guards_at_the_same_time() {
    let root = tempfile::tempdir().unwrap();
    let personal = AppDirs::under(&root.path().join("personal"));
    let business = AppDirs::under(&root.path().join("business"));
    let waker = zapfast::backend::Waker::default();
    let first = single_instance::acquire(&personal.runtime, &waker, "ping");
    let second = single_instance::acquire(&business.runtime, &waker, "ping");
    assert!(matches!(first, single_instance::Outcome::Only(_)));
    assert!(matches!(second, single_instance::Outcome::Only(_)));
    assert!(matches!(
        single_instance::acquire(&business.runtime, &waker, "ping"),
        single_instance::Outcome::Surfaced
    ));
}

#[test]
fn archive_credentials_use_a_separate_service_even_for_the_same_account() {
    let store = keyring_core::mock::Store::new().unwrap();
    let original = store
        .build("rocks.zapfast.ZapFast", "fixture", None)
        .unwrap();
    let business = store
        .build(identity::KEYRING_SERVICE, "fixture", None)
        .unwrap();
    original.set_secret(b"personal fixture").unwrap();
    business.set_secret(b"business fixture").unwrap();
    assert_ne!(identity::KEYRING_SERVICE, "rocks.zapfast.ZapFast");
    assert_eq!(original.get_secret().unwrap(), b"personal fixture");
    assert_eq!(business.get_secret().unwrap(), b"business fixture");
}

#[test]
fn installer_owns_only_business_identity_and_shortcuts() {
    let installer = include_str!("../packaging/windows/zapfast.iss");
    assert!(installer.contains("B52DF836-982D-48F8-91B0-FE4D9E58B7F2"));
    assert!(!installer.contains("F2512314-384A-4002-9933-AB840FD01639"));
    assert!(installer.contains(identity::APPLICATION_ID));
    assert!(!installer.contains("[InstallDelete]"));
    assert!(!installer.contains("FastsApp.lnk"));
    assert!(installer.contains("CloseApplicationsFilter=zapfast-business.exe"));
}
