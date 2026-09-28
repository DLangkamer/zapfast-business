//! Business rejects every upstream helper flag before touching user data.
use std::process::Command;

#[test]
fn upstream_update_helpers_are_rejected() {
    for flag in ["--apply-update", "--update-receipt", "--update-error"] {
        let directory = tempfile::tempdir().unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_zapfast-business"))
            .args([flag, "missing", "--version"])
            .current_dir(directory.path())
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
    }
}
