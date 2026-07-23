#![allow(clippy::unwrap_used)]

#[test]
fn updater_uses_signed_github_release_artifacts() {
    let config: serde_json::Value =
        serde_json::from_str(include_str!("../tauri.conf.json")).expect("base Tauri config");
    let release_config: serde_json::Value =
        serde_json::from_str(include_str!("../tauri.release.conf.json"))
            .expect("release Tauri config");

    assert_eq!(
        config["plugins"]["updater"]["endpoints"],
        serde_json::json!([
            "https://github.com/ducnguyen67201/Goalbar/releases/latest/download/latest.json"
        ])
    );
    assert_eq!(
        config["plugins"]["updater"]["pubkey"],
        serde_json::json!(""),
        "the release build injects the public key without committing signing credentials"
    );
    assert_eq!(
        release_config["bundle"]["createUpdaterArtifacts"],
        serde_json::json!(true)
    );
}

#[test]
fn release_workflow_requires_signing_material_and_both_macos_architectures() {
    let workflow = include_str!("../../.github/workflows/release.yml");

    for required in [
        "GOALBAR_UPDATER_PUBLIC_KEY",
        "TAURI_SIGNING_PRIVATE_KEY",
        "aarch64-apple-darwin",
        "x86_64-apple-darwin",
        "tauri-apps/tauri-action@v1",
        "tauri.release.conf.json",
    ] {
        assert!(
            workflow.contains(required),
            "release workflow must contain {required}"
        );
    }
}
