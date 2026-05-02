use serde::{de::DeserializeOwned, Serialize};

use super::{keys, InventoryDb};
use crate::model::{db_error, CommandResult, InventoryEntry};

impl InventoryDb {
    pub(super) fn put_json<T: Serialize>(&self, key: &[u8], value: &T) -> CommandResult<bool> {
        let bytes = serde_json::to_vec(value).map_err(db_error)?;
        self.store.insert(key, &bytes).map_err(db_error)
    }

    pub(super) fn put_bytes(&self, key: &[u8], value: &[u8]) -> CommandResult<()> {
        self.store.insert(key, value).map_err(db_error)?;
        Ok(())
    }

    pub(super) fn get_json<T: DeserializeOwned>(&self, key: &[u8]) -> CommandResult<Option<T>> {
        if !self.store.contains_key(key) {
            return Ok(None);
        }

        let value = self.store.get(key).map_err(db_error)?;
        serde_json::from_slice(&value).map(Some).map_err(db_error)
    }

    pub(super) fn get_string(&self, key: &[u8]) -> CommandResult<Option<String>> {
        if !self.store.contains_key(key) {
            return Ok(None);
        }

        let value = self.store.get(key).map_err(db_error)?;
        String::from_utf8(value).map(Some).map_err(db_error)
    }

    // Used by schema metadata helpers that are exercised by tests and migrations.
    #[allow(dead_code)]
    pub(super) fn get_u32(&self, key: &[u8], label: &str) -> CommandResult<Option<u32>> {
        self.get_string(key)?
            .map(|value| {
                value
                    .parse::<u32>()
                    .map_err(|error| format!("invalid {label}: {error}"))
            })
            .transpose()
    }

    pub(super) fn put_u32(&self, key: &[u8], label: &str, value: u32) -> CommandResult<()> {
        if value == 0 {
            return Err(format!("{label} must be greater than zero"));
        }

        self.put_bytes(key, value.to_string().as_bytes())
    }

    pub(super) fn get_u64(&self, key: &[u8], label: &str) -> CommandResult<Option<u64>> {
        self.get_string(key)?
            .map(|value| {
                value
                    .parse::<u64>()
                    .map_err(|error| format!("invalid {label}: {error}"))
            })
            .transpose()
    }

    pub(super) fn delete_key(&self, key: &[u8]) -> CommandResult<()> {
        if self.store.contains_key(key) {
            self.store.delete(key).map_err(db_error)?;
        }

        Ok(())
    }

    pub(super) fn scan_sync_prefix_from<F>(
        &self,
        prefix: &str,
        mut start_key: Vec<u8>,
        limit: usize,
        mut visit: F,
    ) -> CommandResult<()>
    where
        F: FnMut(&[u8], &[u8]) -> CommandResult<bool>,
    {
        if limit == 0 {
            return Ok(());
        }

        let range_end = keys::range_end_for_prefix(prefix);
        let prefix_bytes = prefix.as_bytes();
        let mut remaining = limit;

        loop {
            let batch_limit = remaining.min(keys::SYNC_SCAN_BATCH_LIMIT);
            let batch = self
                .store
                .range_query(&start_key, &range_end, batch_limit)
                .map_err(db_error)?;
            if batch.is_empty() {
                return Ok(());
            }

            let is_last_batch = batch.len() < batch_limit;
            let last_key = batch.last().map(|(key, _)| key.clone());

            for (key, value) in batch {
                if !key.starts_with(prefix_bytes) {
                    return Ok(());
                }

                if !visit(&key, &value)? {
                    return Ok(());
                }

                remaining -= 1;
                if remaining == 0 {
                    return Ok(());
                }
            }

            if is_last_batch {
                return Ok(());
            }

            if let Some(last_key) = last_key {
                start_key = keys::next_entry_range_start(last_key);
            }
        }
    }
}

pub(super) fn decode_entry(value: &[u8]) -> CommandResult<InventoryEntry> {
    serde_json::from_slice(value).map_err(db_error)
}

pub(super) fn decode_u64_value(value: &[u8], label: &str) -> CommandResult<u64> {
    let value = String::from_utf8(value.to_vec()).map_err(db_error)?;
    let local_seq = value
        .parse::<u64>()
        .map_err(|error| format!("invalid {label}: {error}"))?;
    keys::validate_local_seq(local_seq)?;
    Ok(local_seq)
}
