use std::path::PathBuf;

use tauri::AppHandle;
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;
use url::Url;

use crate::model::CommandResult;

const IMAGE_FILTER_EXTENSIONS: &[&str] =
    &["png", "jpg", "jpeg", "webp", "gif", "bmp", "tif", "tiff"];

#[tauri::command]
pub(crate) fn open_external(url: String, app: AppHandle) -> CommandResult<bool> {
    let Ok(url) = normalize_external_url(&url) else {
        return Ok(false);
    };

    Ok(app.opener().open_url(url, None::<&str>).is_ok())
}

#[tauri::command]
pub(crate) fn open_path(path: String, app: AppHandle) -> CommandResult<bool> {
    let Ok(path) = normalize_local_path(&path) else {
        return Ok(false);
    };

    if !path.exists() {
        return Ok(false);
    }

    Ok(app
        .opener()
        .open_path(path.to_string_lossy().to_string(), None::<&str>)
        .is_ok())
}

#[tauri::command]
pub(crate) async fn pick_picture_path(app: AppHandle) -> CommandResult<Option<String>> {
    let selected = app
        .dialog()
        .file()
        .set_title("Select Inventory Picture")
        .add_filter("Images", IMAGE_FILTER_EXTENSIONS)
        .blocking_pick_file();

    selected
        .map(|file_path| {
            file_path
                .simplified()
                .into_path()
                .map(|path| path.to_string_lossy().to_string())
                .map_err(|error| format!("Could not read the selected picture path: {error}"))
        })
        .transpose()
}

fn normalize_external_url(value: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() || is_windows_local_path(trimmed) {
        return Err("Invalid external URL.".to_string());
    }

    let parsed = Url::parse(trimmed).map_err(|_| "Invalid external URL.".to_string())?;
    match parsed.scheme() {
        "http" | "https" | "mailto" => Ok(parsed.to_string()),
        _ => Err("Unsupported external URL protocol.".to_string()),
    }
}

fn normalize_local_path(value: &str) -> Result<PathBuf, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() || looks_like_url(trimmed) {
        return Err("Invalid local path.".to_string());
    }

    let path = PathBuf::from(trimmed);
    if !path.is_absolute() {
        return Err("Local path must be absolute.".to_string());
    }

    Ok(path)
}

fn looks_like_url(value: &str) -> bool {
    if is_windows_local_path(value) {
        return false;
    }

    let Some((scheme, _)) = value.split_once(':') else {
        return false;
    };

    !scheme.is_empty()
        && scheme.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '+' | '-' | '.')
        })
}

fn is_windows_local_path(value: &str) -> bool {
    let bytes = value.as_bytes();
    value.starts_with(r"\\")
        || (bytes.len() >= 3
            && bytes[0].is_ascii_alphabetic()
            && bytes[1] == b':'
            && matches!(bytes[2], b'\\' | b'/'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn external_urls_allow_only_safe_protocols() {
        assert_eq!(
            normalize_external_url(" https://example.com/path ").unwrap(),
            "https://example.com/path"
        );
        assert_eq!(
            normalize_external_url("mailto:inventory@example.com").unwrap(),
            "mailto:inventory@example.com"
        );
        assert!(normalize_external_url("javascript:alert(1)").is_err());
        assert!(normalize_external_url("file:///C:/Pictures/item.jpg").is_err());
        assert!(normalize_external_url(r"C:\Pictures\item.jpg").is_err());
        assert!(normalize_external_url(r"\\server\share\item.jpg").is_err());
        assert!(normalize_external_url("example.com").is_err());
    }

    #[cfg(windows)]
    #[test]
    fn local_paths_allow_absolute_windows_paths() {
        assert_eq!(
            normalize_local_path(r" C:\Pictures\item.jpg ").unwrap(),
            PathBuf::from(r"C:\Pictures\item.jpg")
        );
        assert_eq!(
            normalize_local_path(r"\\server\share\item.jpg").unwrap(),
            PathBuf::from(r"\\server\share\item.jpg")
        );
    }

    #[test]
    fn local_paths_reject_urls_and_relative_paths() {
        assert!(normalize_local_path("").is_err());
        assert!(normalize_local_path("https://example.com/item.jpg").is_err());
        assert!(normalize_local_path("file:///C:/Pictures/item.jpg").is_err());
        assert!(normalize_local_path("Pictures/item.jpg").is_err());
    }
}
