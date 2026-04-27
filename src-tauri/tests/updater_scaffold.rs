#[path = "../src/updater.rs"]
mod updater;

#[test]
fn scaffold_error_commands_serialize_frontend_contract() {
    let download = serde_json::to_value(updater::download_update().unwrap()).unwrap();
    assert_eq!(download["available"], serde_json::json!(false));
    assert_eq!(download["status"], serde_json::json!("error"));
    assert!(download["error"]
        .as_str()
        .unwrap_or_default()
        .contains("not configured"));
    assert!(download.get("downloadPhase").is_none());
    assert!(download.get("downloadProgress").is_none());
    assert!(download.get("downloadedInstallerPath").is_none());

    let install = serde_json::to_value(updater::install_update().unwrap()).unwrap();
    assert_eq!(install["available"], serde_json::json!(false));
    assert_eq!(install["status"], serde_json::json!("error"));
    assert!(install["error"]
        .as_str()
        .unwrap_or_default()
        .contains("no verified updater download is available"));
    assert!(install.get("installerPid").is_none());
    assert!(install.get("installLogPath").is_none());
}
