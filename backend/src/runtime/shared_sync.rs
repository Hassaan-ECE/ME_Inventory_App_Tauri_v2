use std::sync::{Arc, Mutex};

use crate::model::{CommandResult, InventorySharedStatus};

/// Serializes every backend critical section that can change inventory rows,
/// local sync metadata, operation files, or shared snapshot state.
///
/// Mutations take the same gate as manual sync and background publish because
/// entry records, outbox operations, applied markers, tombstones, and revisions
/// are committed as one recoverable unit.
#[derive(Clone, Default)]
pub(crate) struct SharedSyncCoordinator {
    gate: Arc<Mutex<()>>,
    last_background_status: Arc<Mutex<Option<InventorySharedStatus>>>,
}

impl SharedSyncCoordinator {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn run_exclusive<T>(
        &self,
        operation_name: &str,
        operation: impl FnOnce() -> CommandResult<T>,
    ) -> CommandResult<T> {
        let _guard = self.gate.lock().map_err(|_| {
            format!("Shared sync coordinator is unavailable during {operation_name}.")
        })?;
        operation()
    }

    pub(crate) fn set_background_status(&self, status: InventorySharedStatus) -> CommandResult<()> {
        let mut last_background_status = self
            .last_background_status
            .lock()
            .map_err(|_| "Shared sync status state is unavailable.".to_string())?;
        *last_background_status = Some(status);
        Ok(())
    }

    pub(crate) fn background_status(&self) -> CommandResult<Option<InventorySharedStatus>> {
        let last_background_status = self
            .last_background_status
            .lock()
            .map_err(|_| "Shared sync status state is unavailable.".to_string())?;
        Ok(last_background_status.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        },
        thread,
        time::Duration,
    };

    #[test]
    fn shared_sync_operations_are_serialized() {
        let coordinator = SharedSyncCoordinator::new();
        let active = Arc::new(AtomicUsize::new(0));
        let max_active = Arc::new(AtomicUsize::new(0));
        let mut handles = Vec::new();

        for _ in 0..4 {
            let coordinator = coordinator.clone();
            let active = Arc::clone(&active);
            let max_active = Arc::clone(&max_active);
            handles.push(thread::spawn(move || {
                coordinator
                    .run_exclusive("test", || {
                        let current = active.fetch_add(1, Ordering::SeqCst) + 1;
                        max_active.fetch_max(current, Ordering::SeqCst);
                        thread::sleep(Duration::from_millis(5));
                        active.fetch_sub(1, Ordering::SeqCst);
                        Ok(())
                    })
                    .unwrap();
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(max_active.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn manual_sync_and_mutation_triggered_publish_share_the_same_gate() {
        let coordinator = SharedSyncCoordinator::new();
        let events = Arc::new(Mutex::new(Vec::new()));
        let (started_tx, started_rx) = std::sync::mpsc::channel();
        let manual_coordinator = coordinator.clone();
        let manual_events = Arc::clone(&events);

        let manual_sync = thread::spawn(move || {
            manual_coordinator
                .run_exclusive("manual sync", || {
                    manual_events.lock().unwrap().push("manual-start");
                    started_tx.send(()).unwrap();
                    thread::sleep(Duration::from_millis(10));
                    manual_events.lock().unwrap().push("manual-end");
                    Ok(())
                })
                .unwrap();
        });

        started_rx.recv().unwrap();
        coordinator
            .run_exclusive("mutation-triggered publish", || {
                events.lock().unwrap().push("publish");
                Ok(())
            })
            .unwrap();
        manual_sync.join().unwrap();

        assert_eq!(
            events.lock().unwrap().as_slice(),
            ["manual-start", "manual-end", "publish"]
        );
    }

    #[test]
    fn background_publish_status_is_available_to_later_loads() {
        let coordinator = SharedSyncCoordinator::new();
        let status = InventorySharedStatus {
            available: false,
            can_modify: true,
            enabled: true,
            has_local_only_changes: Some(true),
            last_snapshot_id: None,
            message: "Background shared publish failed: disk full".to_string(),
            mutation_mode: "local".to_string(),
            revision: Some("3".to_string()),
            shared_root_path: Some("S:\\ME".to_string()),
            sync_interval_ms: Some(500),
        };

        coordinator.set_background_status(status).unwrap();

        let stored = coordinator.background_status().unwrap().unwrap();
        assert_eq!(
            stored.message,
            "Background shared publish failed: disk full"
        );
        assert_eq!(stored.has_local_only_changes, Some(true));
        assert_eq!(stored.mutation_mode, "local");
    }
}
