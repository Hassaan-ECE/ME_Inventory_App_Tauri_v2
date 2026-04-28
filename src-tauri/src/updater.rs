use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Manager};

pub(crate) type UpdaterCommandResult<T> = Result<T, String>;

const UPDATE_ROOT_ENV: &str = "ME_INVENTORY_UPDATE_ROOT";
const DEFAULT_UPDATE_ROOT: &str = r"S:\Manufacturing\Internal\_Syed_H_Shah\InventoryApps\ME";
const MANIFEST_FILE: &str = "current.json";

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

#[derive(Debug, Clone, Deserialize)]
struct UpdateManifest {
    version: String,
    installer_path: String,
    sha256: String,
    notes: Option<String>,
    published_at: Option<String>,
}

#[tauri::command]
pub(crate) fn check_for_update() -> UpdaterCommandResult<UpdateState> {
    check_for_update_at_root(&resolve_update_root())
}

#[tauri::command]
pub(crate) fn download_update(app: AppHandle) -> UpdaterCommandResult<UpdateState> {
    let root = resolve_update_root();
    let cache_dir = update_cache_dir(&app)?;
    download_update_to_cache(&root, &cache_dir)
}

#[tauri::command]
pub(crate) fn install_update(app: AppHandle) -> UpdaterCommandResult<UpdateState> {
    let root = resolve_update_root();
    let cache_dir = update_cache_dir(&app)?;
    install_update_from_cache(&root, &cache_dir)
}

fn check_for_update_at_root(root: &Path) -> UpdaterCommandResult<UpdateState> {
    let manifest = match read_manifest(root) {
        Ok(manifest) => manifest,
        Err(error) => {
            return Ok(not_available_state(Some(format!(
                "No update is available. {error}"
            ))));
        }
    };

    if !version_is_newer(&manifest.version, &current_version()) {
        return Ok(UpdateState {
            latest_version: Some(manifest.version),
            notes: manifest.notes,
            published_at: manifest.published_at,
            ..not_available_state(Some("ME Inventory is up to date.".to_string()))
        });
    }

    Ok(available_state(&manifest, UpdateStatus::Available))
}

fn download_update_to_cache(root: &Path, cache_dir: &Path) -> UpdaterCommandResult<UpdateState> {
    let manifest = latest_manifest(root)?;
    let installer_path = resolve_installer_path(root, &manifest.installer_path);
    if !installer_path.exists() {
        return Ok(error_state(
            &manifest,
            format!(
                "Update installer was not found: {}",
                installer_path.display()
            ),
        ));
    }

    fs::create_dir_all(cache_dir).map_err(|error| {
        format!(
            "Could not create update cache folder {}: {error}",
            cache_dir.display()
        )
    })?;
    let cached_path = cached_installer_path(cache_dir, &manifest.version);
    fs::copy(&installer_path, &cached_path).map_err(|error| {
        format!(
            "Could not copy update installer from {} to {}: {error}",
            installer_path.display(),
            cached_path.display()
        )
    })?;

    if let Err(error) = verify_installer_hash(&cached_path, &manifest.sha256) {
        let _ = fs::remove_file(&cached_path);
        return Ok(error_state(&manifest, error));
    }

    Ok(UpdateState {
        download_phase: Some(DownloadPhase::Ready),
        download_progress: Some(100.0),
        downloaded_installer_path: Some(cached_path.to_string_lossy().into_owned()),
        status: UpdateStatus::Ready,
        ..available_state(&manifest, UpdateStatus::Ready)
    })
}

fn install_update_from_cache(root: &Path, cache_dir: &Path) -> UpdaterCommandResult<UpdateState> {
    let manifest = latest_manifest(root)?;
    let cached_path = cached_installer_path(cache_dir, &manifest.version);
    if !cached_path.exists() {
        return Ok(error_state(
            &manifest,
            "Download the update before installing it.".to_string(),
        ));
    }
    if let Err(error) = verify_installer_hash(&cached_path, &manifest.sha256) {
        return Ok(error_state(&manifest, error));
    }

    let child = Command::new(&cached_path).spawn().map_err(|error| {
        format!(
            "Could not open update installer {}: {error}",
            cached_path.display()
        )
    })?;

    Ok(UpdateState {
        downloaded_installer_path: Some(cached_path.to_string_lossy().into_owned()),
        installer_pid: Some(child.id()),
        status: UpdateStatus::Installing,
        ..available_state(&manifest, UpdateStatus::Installing)
    })
}

fn latest_manifest(root: &Path) -> UpdaterCommandResult<UpdateManifest> {
    let manifest = read_manifest(root)?;
    if !version_is_newer(&manifest.version, &current_version()) {
        return Err("ME Inventory is already up to date.".to_string());
    }
    Ok(manifest)
}

fn read_manifest(root: &Path) -> UpdaterCommandResult<UpdateManifest> {
    let path = root.join(MANIFEST_FILE);
    let text = fs::read_to_string(&path)
        .map_err(|error| format!("Could not read update manifest {}: {error}", path.display()))?;
    serde_json::from_str(&text).map_err(|error| {
        format!(
            "Could not parse update manifest {}: {error}",
            path.display()
        )
    })
}

fn resolve_update_root() -> PathBuf {
    env::var_os(UPDATE_ROOT_ENV)
        .and_then(|value| {
            let path = value.to_string_lossy().trim().to_string();
            (!path.is_empty()).then_some(PathBuf::from(path))
        })
        .unwrap_or_else(|| PathBuf::from(DEFAULT_UPDATE_ROOT))
}

fn resolve_installer_path(root: &Path, installer_path: &str) -> PathBuf {
    let path = PathBuf::from(installer_path);
    if path.is_absolute() {
        path
    } else {
        root.join(path)
    }
}

fn update_cache_dir(app: &AppHandle) -> UpdaterCommandResult<PathBuf> {
    app.path()
        .app_cache_dir()
        .map(|path| path.join("updates"))
        .map_err(|error| format!("Could not resolve update cache folder: {error}"))
}

fn cached_installer_path(cache_dir: &Path, version: &str) -> PathBuf {
    cache_dir.join(format!("ME Inventory Setup {version}.exe"))
}

fn verify_installer_hash(path: &Path, expected_hash: &str) -> Result<(), String> {
    let actual_hash = file_sha256(path)?;
    if actual_hash.eq_ignore_ascii_case(expected_hash.trim()) {
        return Ok(());
    }

    Err(format!(
        "Update installer hash mismatch. Expected {}, got {}.",
        expected_hash.trim(),
        actual_hash
    ))
}

fn file_sha256(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path)
        .map_err(|error| format!("Could not read installer {}: {error}", path.display()))?;
    let digest = Sha256::digest(bytes);
    Ok(hex_lower(&digest))
}

fn hex_lower(bytes: &[u8]) -> String {
    let mut hex = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        hex.push(nibble_to_hex(byte >> 4));
        hex.push(nibble_to_hex(byte & 0x0f));
    }
    hex
}

fn nibble_to_hex(nibble: u8) -> char {
    match nibble & 0x0f {
        value @ 0..=9 => (b'0' + value) as char,
        value => (b'a' + (value - 10)) as char,
    }
}

fn version_is_newer(latest: &str, current: &str) -> bool {
    let latest_parts = version_parts(latest);
    let current_parts = version_parts(current);
    let len = latest_parts.len().max(current_parts.len());

    for index in 0..len {
        let latest_part = latest_parts.get(index).copied().unwrap_or(0);
        let current_part = current_parts.get(index).copied().unwrap_or(0);
        if latest_part > current_part {
            return true;
        }
        if latest_part < current_part {
            return false;
        }
    }

    false
}

fn version_parts(version: &str) -> Vec<u64> {
    version
        .split('.')
        .map(|part| {
            part.chars()
                .take_while(|character| character.is_ascii_digit())
                .collect::<String>()
                .parse::<u64>()
                .unwrap_or(0)
        })
        .collect()
}

fn available_state(manifest: &UpdateManifest, status: UpdateStatus) -> UpdateState {
    UpdateState {
        available: true,
        current_version: current_version(),
        download_phase: None,
        download_progress: None,
        downloaded_installer_path: None,
        error: None,
        install_log_path: None,
        installer_pid: None,
        latest_version: Some(manifest.version.clone()),
        notes: manifest.notes.clone(),
        published_at: manifest.published_at.clone(),
        status,
    }
}

fn not_available_state(notes: Option<String>) -> UpdateState {
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
        notes,
        published_at: None,
        status: UpdateStatus::NotAvailable,
    }
}

fn error_state(manifest: &UpdateManifest, message: String) -> UpdateState {
    UpdateState {
        available: true,
        current_version: current_version(),
        download_phase: None,
        download_progress: None,
        downloaded_installer_path: None,
        error: Some(message),
        install_log_path: None,
        installer_pid: None,
        latest_version: Some(manifest.version.clone()),
        notes: manifest.notes.clone(),
        published_at: manifest.published_at.clone(),
        status: UpdateStatus::Error,
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
    fn version_compare_detects_newer_versions() {
        assert!(version_is_newer("0.9.6", "0.9.5"));
        assert!(version_is_newer("0.10.0", "0.9.9"));
        assert!(!version_is_newer("0.9.5", "0.9.5"));
        assert!(!version_is_newer("0.9.4", "0.9.5"));
    }

    #[test]
    fn check_for_update_reports_newer_shared_manifest() {
        let root = unique_test_dir("update-check");
        fs::create_dir_all(&root).unwrap();
        write_manifest(&root, "99.0.0", "installer.exe", "abc");

        let state = check_for_update_at_root(&root).unwrap();

        assert!(state.available);
        assert_eq!(state.latest_version.as_deref(), Some("99.0.0"));
        assert_eq!(state.status, UpdateStatus::Available);
    }

    #[test]
    fn check_for_update_stays_quiet_when_manifest_is_current() {
        let root = unique_test_dir("update-current");
        fs::create_dir_all(&root).unwrap();
        write_manifest(&root, env!("CARGO_PKG_VERSION"), "installer.exe", "abc");

        let state = check_for_update_at_root(&root).unwrap();

        assert!(!state.available);
        assert_eq!(state.status, UpdateStatus::NotAvailable);
    }

    #[test]
    fn download_copies_installer_and_verifies_hash() {
        let root = unique_test_dir("update-download");
        let cache = root.join("cache");
        let release_dir = root.join("releases").join("99.0.0");
        fs::create_dir_all(&release_dir).unwrap();
        let installer = release_dir.join("ME Inventory Setup 99.0.0.exe");
        fs::write(&installer, b"fake-installer").unwrap();
        let hash = file_sha256(&installer).unwrap();
        write_manifest(
            &root,
            "99.0.0",
            "releases/99.0.0/ME Inventory Setup 99.0.0.exe",
            &hash,
        );

        let state = download_update_to_cache(&root, &cache).unwrap();

        assert_eq!(state.status, UpdateStatus::Ready);
        let cached_path = state.downloaded_installer_path.unwrap();
        assert_eq!(fs::read(cached_path).unwrap(), b"fake-installer");
    }

    #[test]
    fn download_rejects_hash_mismatch() {
        let root = unique_test_dir("update-hash-mismatch");
        let cache = root.join("cache");
        fs::create_dir_all(&root).unwrap();
        let installer = root.join("installer.exe");
        fs::write(&installer, b"fake-installer").unwrap();
        write_manifest(&root, "99.0.0", "installer.exe", "bad");

        let state = download_update_to_cache(&root, &cache).unwrap();

        assert_eq!(state.status, UpdateStatus::Error);
        assert!(state
            .error
            .as_deref()
            .unwrap_or_default()
            .contains("hash mismatch"));
    }

    #[test]
    fn serialized_state_matches_frontend_update_state_shape() {
        let root = unique_test_dir("update-serialize");
        fs::create_dir_all(&root).unwrap();
        write_manifest(&root, "99.0.0", "installer.exe", "abc");
        let value = serde_json::to_value(check_for_update_at_root(&root).unwrap()).unwrap();

        assert_eq!(value["available"], json!(true));
        assert_eq!(value["currentVersion"], json!(env!("CARGO_PKG_VERSION")));
        assert_eq!(value["status"], json!("available"));
        assert_eq!(value["latestVersion"], json!("99.0.0"));
        assert!(value.get("installerPid").is_none());
    }

    fn write_manifest(root: &Path, version: &str, installer_path: &str, sha256: &str) {
        let manifest = json!({
            "version": version,
            "installer_path": installer_path,
            "sha256": sha256,
            "notes": "Test release",
            "published_at": "2026-04-27T00:00:00-05:00"
        });
        fs::write(root.join(MANIFEST_FILE), manifest.to_string()).unwrap();
    }

    fn unique_test_dir(prefix: &str) -> PathBuf {
        env::temp_dir().join(format!("{prefix}-{}", uuid::Uuid::new_v4().simple()))
    }
}
