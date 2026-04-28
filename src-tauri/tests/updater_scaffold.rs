#[path = "../src/updater.rs"]
mod updater;

#[test]
fn check_for_update_serializes_frontend_contract() {
    let state = serde_json::to_value(updater::check_for_update().unwrap()).unwrap();
    assert_eq!(state["currentVersion"], serde_json::json!(env!("CARGO_PKG_VERSION")));
    assert!(state.get("available").is_some());
    assert!(state.get("status").is_some());
    assert!(state.get("downloadedInstallerPath").is_none());
    assert!(state.get("installerPid").is_none());
    assert!(state.get("installLogPath").is_none());
}
