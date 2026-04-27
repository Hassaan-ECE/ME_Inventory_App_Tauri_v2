use std::{
    collections::hash_map::DefaultHasher,
    fs,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;
use url::Url;

use crate::model::CommandResult;

const IMAGE_FILTER_EXTENSIONS: &[&str] =
    &["png", "jpg", "jpeg", "webp", "gif", "bmp", "tif", "tiff"];
const MAX_PREVIEW_SOURCE_BYTES: u64 = 50 * 1024 * 1024;
const PREVIEW_CACHE_DIR_NAME: &str = "picture-previews";

#[tauri::command]
pub(crate) fn open_external(url: String, app: AppHandle) -> CommandResult<bool> {
    let Ok(url) = normalize_external_url(&url) else {
        return Ok(false);
    };

    Ok(app.opener().open_url(url, None::<&str>).is_ok())
}

#[tauri::command]
pub(crate) fn open_path(path: String, app: AppHandle) -> CommandResult<bool> {
    let Ok(path) = normalize_picture_path(&path) else {
        return Ok(false);
    };

    if !path.is_file() {
        return Ok(false);
    }

    Ok(app
        .opener()
        .open_path(path.to_string_lossy().to_string(), None::<&str>)
        .is_ok())
}

#[tauri::command]
pub(crate) fn load_picture_preview(path: String, app: AppHandle) -> CommandResult<Option<String>> {
    let cache_root = app
        .path()
        .app_cache_dir()
        .map_err(|error| error.to_string())?;
    build_picture_preview_file(&path, &cache_root)
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

fn build_picture_preview_file(value: &str, cache_root: &Path) -> CommandResult<Option<String>> {
    let Ok(path) = normalize_picture_path(value) else {
        return Ok(None);
    };

    if !path.is_file() {
        return Ok(None);
    }

    let metadata = fs::metadata(&path).map_err(|error| error.to_string())?;
    if metadata.len() > MAX_PREVIEW_SOURCE_BYTES {
        return Ok(None);
    }

    let preview_cache_dir = cache_root.join(PREVIEW_CACHE_DIR_NAME);
    fs::create_dir_all(&preview_cache_dir).map_err(|error| error.to_string())?;

    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("img")
        .to_ascii_lowercase();
    let fingerprint = picture_preview_fingerprint(&path, &metadata);
    let destination = preview_cache_dir.join(format!("{fingerprint:016x}.{extension}"));

    if !destination.is_file() {
        let temp_destination = preview_cache_dir.join(format!(
            "{fingerprint:016x}.tmp-{}",
            uuid::Uuid::new_v4().simple()
        ));

        fs::copy(&path, &temp_destination).map_err(|error| error.to_string())?;
        if let Err(error) = fs::rename(&temp_destination, &destination) {
            if destination.is_file() {
                let _ = fs::remove_file(&temp_destination);
            } else {
                let _ = fs::remove_file(&temp_destination);
                return Err(error.to_string());
            }
        }
    }

    Ok(Some(destination.to_string_lossy().to_string()))
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

fn normalize_picture_path(value: &str) -> Result<PathBuf, String> {
    let path = normalize_local_path(value)?;
    let Some(extension) = path.extension().and_then(|extension| extension.to_str()) else {
        return Err("Picture path must use a supported image extension.".to_string());
    };

    if IMAGE_FILTER_EXTENSIONS
        .iter()
        .any(|allowed| extension.eq_ignore_ascii_case(allowed))
    {
        Ok(path)
    } else {
        Err("Picture path must use a supported image extension.".to_string())
    }
}

fn picture_preview_fingerprint(path: &Path, metadata: &fs::Metadata) -> u64 {
    let mut hasher = DefaultHasher::new();
    path.to_string_lossy()
        .to_ascii_lowercase()
        .hash(&mut hasher);
    metadata.len().hash(&mut hasher);

    if let Ok(modified) = metadata.modified() {
        if let Ok(duration) = modified.duration_since(UNIX_EPOCH) {
            duration.as_nanos().hash(&mut hasher);
        }
    }

    hasher.finish()
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
        assert!(normalize_external_url("C:/Pictures/item.jpg").is_err());
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
        assert!(normalize_local_path("mailto:inventory@example.com").is_err());
        assert!(normalize_local_path(r"C:Pictures\item.jpg").is_err());
        assert!(normalize_local_path("Pictures/item.jpg").is_err());
    }

    #[cfg(windows)]
    #[test]
    fn picture_paths_allow_only_supported_image_extensions() {
        assert_eq!(
            normalize_picture_path(r" C:\Pictures\item.JPG ").unwrap(),
            PathBuf::from(r"C:\Pictures\item.JPG")
        );
        assert!(normalize_picture_path(r"C:\Pictures\item.ps1").is_err());
        assert!(normalize_picture_path(r"C:\Pictures\item.exe").is_err());
        assert!(normalize_picture_path(r"C:\Pictures\item").is_err());
        assert!(normalize_picture_path("https://example.com/item.jpg").is_err());
    }

    #[test]
    fn preview_cache_rejects_invalid_or_missing_paths() {
        let cache_dir = temp_test_dir("me-inventory-preview-cache-invalid");

        assert_eq!(
            build_picture_preview_file("https://example.com/item.png", &cache_dir).unwrap(),
            None
        );
        assert_eq!(
            build_picture_preview_file("relative.png", &cache_dir).unwrap(),
            None
        );
        assert_eq!(
            build_picture_preview_file(r"C:\missing.ps1", &cache_dir).unwrap(),
            None
        );
        assert_eq!(
            build_picture_preview_file(r"C:\definitely-missing-picture.png", &cache_dir).unwrap(),
            None
        );

        let _ = fs::remove_dir_all(cache_dir);
    }

    #[test]
    fn preview_cache_copies_valid_image() {
        let path = std::env::temp_dir().join(format!(
            "me-inventory-preview-{}.png",
            uuid::Uuid::new_v4().simple()
        ));
        let cache_dir = temp_test_dir("me-inventory-preview-cache-valid");
        fs::write(&path, [0x89, b'P', b'N', b'G']).unwrap();

        let preview = build_picture_preview_file(path.to_string_lossy().as_ref(), &cache_dir)
            .unwrap()
            .expect("valid image should create a cached preview");
        let preview_path = PathBuf::from(preview);

        assert!(preview_path.starts_with(cache_dir.join(PREVIEW_CACHE_DIR_NAME)));
        assert_eq!(fs::read(preview_path).unwrap(), [0x89, b'P', b'N', b'G']);

        let _ = fs::remove_file(path);
        let _ = fs::remove_dir_all(cache_dir);
    }

    #[test]
    fn preview_cache_allows_common_camera_sized_images() {
        let path = std::env::temp_dir().join(format!(
            "me-inventory-preview-camera-{}.jpg",
            uuid::Uuid::new_v4().simple()
        ));
        let cache_dir = temp_test_dir("me-inventory-preview-cache-camera");
        let file = fs::File::create(&path).unwrap();
        file.set_len(3 * 1024 * 1024).unwrap();
        drop(file);

        let preview = build_picture_preview_file(path.to_string_lossy().as_ref(), &cache_dir)
            .unwrap()
            .expect("common camera-sized image should create a cached preview");

        assert!(PathBuf::from(preview).is_file());

        let _ = fs::remove_file(path);
        let _ = fs::remove_dir_all(cache_dir);
    }

    #[test]
    fn preview_cache_rejects_oversized_images() {
        let path = std::env::temp_dir().join(format!(
            "me-inventory-preview-large-{}.png",
            uuid::Uuid::new_v4().simple()
        ));
        let cache_dir = temp_test_dir("me-inventory-preview-cache-large");
        let file = fs::File::create(&path).unwrap();
        file.set_len(MAX_PREVIEW_SOURCE_BYTES + 1).unwrap();
        drop(file);

        let preview =
            build_picture_preview_file(path.to_string_lossy().as_ref(), &cache_dir).unwrap();

        assert_eq!(preview, None);

        let _ = fs::remove_file(path);
        let _ = fs::remove_dir_all(cache_dir);
    }

    fn temp_test_dir(prefix: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("{prefix}-{}", uuid::Uuid::new_v4().simple()));
        fs::create_dir_all(&path).unwrap();
        path
    }
}
