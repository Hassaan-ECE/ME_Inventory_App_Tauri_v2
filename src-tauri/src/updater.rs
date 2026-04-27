use serde::Serialize;

pub(crate) type UpdaterCommandResult<T> = Result<T, String>;

const NOT_CONFIGURED_MESSAGE: &str =
    "Updater is not configured for this build. Add real signed Tauri updater configuration before enabling update checks.";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
#[allow(dead_code)]
pub(crate) enum UpdateStatus {
    Idle,
    Checking,
    Available,
    NotAvailable,
    Downloading,
    Ready,
    Installing,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub(crate) enum DownloadPhase {
    Copying,
    Verifying,
    Ready,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UpdateState {
    pub available: bool,
    pub current_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub download_phase: Option<DownloadPhase>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub download_progress: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub downloaded_installer_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub install_log_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub installer_pid: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub published_at: Option<String>,
    pub status: UpdateStatus,
}

#[tauri::command]
pub(crate) fn check_for_update() -> UpdaterCommandResult<UpdateState> {
    Ok(not_configured_state(UpdateStatus::NotAvailable))
}

#[tauri::command]
pub(crate) fn download_update() -> UpdaterCommandResult<UpdateState> {
    Ok(not_configured_error_state(
        "Cannot download an update because the updater is not configured.",
    ))
}

#[tauri::command]
pub(crate) fn install_update() -> UpdaterCommandResult<UpdateState> {
    Ok(not_configured_error_state(
        "Cannot install an update because no verified updater download is available.",
    ))
}

fn not_configured_state(status: UpdateStatus) -> UpdateState {
    UpdateState {
        available: false,
        current_version: current_version(),
        download_phase: None,
        download_progress: None,
        downloaded_installer_path: None,
        error: None,
        install_log_path: None,
        installer_pid: None,
        latest_version: None,
        notes: Some(NOT_CONFIGURED_MESSAGE.to_string()),
        published_at: None,
        status,
    }
}

fn not_configured_error_state(message: &str) -> UpdateState {
    UpdateState {
        error: Some(message.to_string()),
        status: UpdateStatus::Error,
        ..not_configured_state(UpdateStatus::Error)
    }
}

fn current_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn check_for_update_returns_safe_not_configured_state() {
        let state = check_for_update().unwrap();

        assert!(!state.available);
        assert_eq!(state.current_version, env!("CARGO_PKG_VERSION"));
        assert_eq!(state.status, UpdateStatus::NotAvailable);
        assert!(state.latest_version.is_none());
        assert!(state.downloaded_installer_path.is_none());
        assert!(state.installer_pid.is_none());
        assert!(state.error.is_none());
        assert!(state
            .notes
            .as_deref()
            .unwrap_or_default()
            .contains("not configured"));
    }

    #[test]
    fn download_update_does_not_pretend_success_when_unconfigured() {
        let state = download_update().unwrap();

        assert!(!state.available);
        assert_eq!(state.status, UpdateStatus::Error);
        assert!(state
            .error
            .as_deref()
            .unwrap_or_default()
            .contains("not configured"));
        assert!(state.download_phase.is_none());
        assert!(state.download_progress.is_none());
        assert!(state.downloaded_installer_path.is_none());
        assert!(state.latest_version.is_none());
    }

    #[test]
    fn install_update_does_not_pretend_success_when_unconfigured() {
        let state = install_update().unwrap();

        assert!(!state.available);
        assert_eq!(state.status, UpdateStatus::Error);
        assert!(state
            .error
            .as_deref()
            .unwrap_or_default()
            .contains("no verified updater download is available"));
        assert!(state.installer_pid.is_none());
        assert!(state.install_log_path.is_none());
        assert!(state.downloaded_installer_path.is_none());
    }

    #[test]
    fn serialized_state_matches_frontend_update_state_shape() {
        let value = serde_json::to_value(check_for_update().unwrap()).unwrap();

        assert_eq!(value["available"], json!(false));
        assert_eq!(value["currentVersion"], json!(env!("CARGO_PKG_VERSION")));
        assert_eq!(value["status"], json!("not-available"));
        assert!(value.get("downloadedInstallerPath").is_none());
        assert!(value.get("installerPid").is_none());
    }
}
