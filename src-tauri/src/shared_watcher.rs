use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tauri::{AppHandle, Emitter, Runtime};

use crate::model::CommandResult;

pub(crate) const SHARED_INVENTORY_CHANGED_EVENT: &str = "inventory:shared-changed";
const WATCHER_EMIT_DEBOUNCE: Duration = Duration::from_millis(500);

pub(crate) struct SharedSyncWatcher {
    state: Mutex<SharedSyncWatcherState>,
}

struct SharedSyncWatcherState {
    watched_path: Option<PathBuf>,
    watcher: Option<RecommendedWatcher>,
    last_emit: Arc<Mutex<Option<Instant>>>,
}

impl SharedSyncWatcher {
    pub(crate) fn new() -> Self {
        Self {
            state: Mutex::new(SharedSyncWatcherState {
                watched_path: None,
                watcher: None,
                last_emit: Arc::new(Mutex::new(None)),
            }),
        }
    }

    pub(crate) fn ensure_watching<R: Runtime>(
        &self,
        app: AppHandle<R>,
        ops_dir: &Path,
    ) -> CommandResult<()> {
        self.ensure_watching_with_emit(ops_dir, move || {
            let _ = app.emit(SHARED_INVENTORY_CHANGED_EVENT, ());
        })
    }

    fn ensure_watching_with_emit<F>(&self, ops_dir: &Path, emit: F) -> CommandResult<()>
    where
        F: Fn() + Send + 'static,
    {
        if !ops_dir.exists() {
            return Ok(());
        }

        let normalized_path = normalize_watched_path(ops_dir);
        let mut state = self
            .state
            .lock()
            .map_err(|_| "Shared sync watcher state is unavailable.".to_string())?;
        if state
            .watched_path
            .as_ref()
            .is_some_and(|current| paths_match(current, &normalized_path))
        {
            return Ok(());
        }

        state.watcher.take();
        let last_emit = Arc::clone(&state.last_emit);
        let mut watcher = RecommendedWatcher::new(
            move |result: notify::Result<notify::Event>| {
                let Ok(event) = result else {
                    return;
                };
                if !event_kind_should_emit(&event.kind) {
                    return;
                }
                if should_debounce_emit(&last_emit) {
                    return;
                }
                emit();
            },
            Config::default(),
        )
        .map_err(|error| format!("Could not start shared sync watcher: {error}"))?;

        watcher
            .watch(&normalized_path, RecursiveMode::Recursive)
            .map_err(|error| format!("Could not watch shared sync operations: {error}"))?;
        state.watched_path = Some(normalized_path);
        state.watcher = Some(watcher);
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn watched_path_for_test(&self) -> Option<PathBuf> {
        self.state
            .lock()
            .ok()
            .and_then(|state| state.watched_path.clone())
    }
}

fn event_kind_should_emit(kind: &EventKind) -> bool {
    matches!(
        kind,
        EventKind::Any | EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
    )
}

fn should_debounce_emit(last_emit: &Arc<Mutex<Option<Instant>>>) -> bool {
    let Ok(mut last_emit) = last_emit.lock() else {
        return true;
    };
    if last_emit
        .as_ref()
        .is_some_and(|instant| instant.elapsed() < WATCHER_EMIT_DEBOUNCE)
    {
        return true;
    }
    *last_emit = Some(Instant::now());
    false
}

fn normalize_watched_path(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

fn paths_match(left: &Path, right: &Path) -> bool {
    if cfg!(windows) {
        left.to_string_lossy()
            .eq_ignore_ascii_case(&right.to_string_lossy())
    } else {
        left == right
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_ops_dir_is_ignored() {
        let watcher = SharedSyncWatcher::new();
        let missing_path = std::env::temp_dir().join("me-inventory-missing-ops");

        watcher
            .ensure_watching_with_emit(&missing_path, || {})
            .unwrap();

        assert!(watcher.watched_path_for_test().is_none());
    }

    #[test]
    fn watcher_starts_after_ops_dir_becomes_available() {
        let watcher = SharedSyncWatcher::new();
        let ops_dir = std::env::temp_dir().join(format!(
            "me-inventory-ops-{}",
            uuid::Uuid::new_v4().simple()
        ));

        watcher.ensure_watching_with_emit(&ops_dir, || {}).unwrap();
        assert!(watcher.watched_path_for_test().is_none());

        std::fs::create_dir_all(&ops_dir).unwrap();
        watcher.ensure_watching_with_emit(&ops_dir, || {}).unwrap();
        assert!(watcher.watched_path_for_test().is_some());
    }
}
