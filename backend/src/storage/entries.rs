use std::{cmp::Ordering, collections::HashSet};

use super::{codec::decode_entry, keys, InventoryDb};
use crate::model::{db_error, numeric_id, CommandResult, InventoryEntry};

#[allow(dead_code)]
impl InventoryDb {
    pub(crate) fn has_entries(&self) -> CommandResult<bool> {
        Ok(!self
            .store
            .range_query(
                keys::ENTRY_PREFIX.as_bytes(),
                keys::ENTRY_RANGE_END.as_bytes(),
                1,
            )
            .map_err(db_error)?
            .is_empty())
    }

    pub(crate) fn load_entries(&self) -> CommandResult<Vec<InventoryEntry>> {
        let mut entries = Vec::new();
        self.scan_entries(|entry| {
            entries.push(entry);
            Ok(true)
        })?;

        entries.sort_by(compare_default_entries);
        Ok(entries)
    }

    pub(crate) fn find_entry(&self, entry_id: &str) -> CommandResult<Option<InventoryEntry>> {
        let entry_id = entry_id.trim();
        if entry_id.is_empty() {
            return Ok(None);
        }

        if entry_id.starts_with(keys::ENTRY_PREFIX) {
            if let Some(entry) = self.get_entry_by_key(entry_id.as_bytes())? {
                return Ok(Some(entry));
            }
        }

        if keys::is_numeric_entry_id(entry_id) {
            if let Some(entry) = self.find_entry_by_id(entry_id)? {
                return Ok(Some(entry));
            }
        }

        if let Some(entry) = self.get_entry_by_uuid(entry_id)? {
            return Ok(Some(entry));
        }

        if keys::is_numeric_entry_id(entry_id) {
            Ok(None)
        } else {
            self.find_entry_by_id(entry_id)
        }
    }

    pub(crate) fn put_entry(&self, entry: &InventoryEntry) -> CommandResult<bool> {
        let inserted = self.put_json(keys::entry_key(&entry.entry_uuid).as_bytes(), entry)?;
        self.put_entry_id_index(entry)?;
        Ok(inserted)
    }

    pub(crate) fn delete_entry(&self, entry: &InventoryEntry) -> CommandResult<()> {
        self.store
            .delete(keys::entry_key(&entry.entry_uuid).as_bytes())
            .map_err(db_error)?;
        self.delete_entry_id_index(&entry.id)?;
        Ok(())
    }

    pub(crate) fn replace_entries_snapshot(
        &self,
        entries: &[InventoryEntry],
    ) -> CommandResult<bool> {
        let existing = self.load_entries()?;
        let changed = existing.len() != entries.len()
            || existing.iter().any(|entry| {
                entries
                    .iter()
                    .find(|incoming| incoming.entry_uuid == entry.entry_uuid)
                    .map(|incoming| incoming != entry)
                    .unwrap_or(true)
            });

        for entry in existing {
            self.delete_entry(&entry)?;
        }

        let mut seen_ids = HashSet::new();
        let mut max_id = 0;
        for entry in entries {
            if !seen_ids.insert(entry.entry_uuid.clone()) {
                continue;
            }
            if let Ok(id) = entry.id.parse::<i64>() {
                max_id = max_id.max(id);
            }
            self.put_entry(entry)?;
        }
        self.set_next_entry_id(max_id + 1)?;

        Ok(changed)
    }

    pub(super) fn get_entry_by_uuid(
        &self,
        entry_uuid: &str,
    ) -> CommandResult<Option<InventoryEntry>> {
        self.get_entry_by_key(keys::entry_key(entry_uuid).as_bytes())
    }

    fn get_entry_by_key(&self, key: &[u8]) -> CommandResult<Option<InventoryEntry>> {
        if !self.store.contains_key(key) {
            return Ok(None);
        }

        let value = self.store.get(key).map_err(db_error)?;
        decode_entry(&value).map(Some)
    }

    fn find_entry_by_id(&self, entry_id: &str) -> CommandResult<Option<InventoryEntry>> {
        if let Some(entry) = self.find_entry_by_id_index(entry_id)? {
            return Ok(Some(entry));
        }

        let mut found = None;
        self.scan_entries(|entry| {
            if entry.id == entry_id {
                self.put_entry_id_index(&entry)?;
                found = Some(entry);
                Ok(false)
            } else {
                Ok(true)
            }
        })?;

        Ok(found)
    }

    fn find_entry_by_id_index(&self, entry_id: &str) -> CommandResult<Option<InventoryEntry>> {
        let key = keys::entry_id_key(entry_id);
        if !self.store.contains_key(key.as_bytes()) {
            return Ok(None);
        }

        let uuid_bytes = self.store.get(key.as_bytes()).map_err(db_error)?;
        let entry_uuid = String::from_utf8(uuid_bytes).map_err(db_error)?;
        let Some(entry) = self.get_entry_by_uuid(&entry_uuid)? else {
            return Ok(None);
        };

        Ok((entry.id == entry_id).then_some(entry))
    }

    fn put_entry_id_index(&self, entry: &InventoryEntry) -> CommandResult<()> {
        if entry.id.is_empty() {
            return Ok(());
        }

        self.put_bytes(
            keys::entry_id_key(&entry.id).as_bytes(),
            entry.entry_uuid.as_bytes(),
        )
    }

    fn delete_entry_id_index(&self, entry_id: &str) -> CommandResult<()> {
        if entry_id.is_empty() {
            return Ok(());
        }

        let key = keys::entry_id_key(entry_id);
        if self.store.contains_key(key.as_bytes()) {
            self.store.delete(key.as_bytes()).map_err(db_error)?;
        }

        Ok(())
    }

    pub(super) fn scan_entries<F>(&self, mut visit: F) -> CommandResult<()>
    where
        F: FnMut(InventoryEntry) -> CommandResult<bool>,
    {
        let mut start_key = keys::ENTRY_PREFIX.as_bytes().to_vec();

        loop {
            let batch = self
                .store
                .range_query(
                    &start_key,
                    keys::ENTRY_RANGE_END.as_bytes(),
                    keys::ENTRY_SCAN_BATCH_LIMIT,
                )
                .map_err(db_error)?;
            if batch.is_empty() {
                return Ok(());
            }

            let is_last_batch = batch.len() < keys::ENTRY_SCAN_BATCH_LIMIT;
            let last_key = batch.last().map(|(key, _)| key.clone());

            for (_, value) in batch {
                if !visit(decode_entry(&value)?)? {
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

fn compare_default_entries(left: &InventoryEntry, right: &InventoryEntry) -> Ordering {
    right
        .updated_at
        .cmp(&left.updated_at)
        .then_with(|| numeric_id(&right.id).cmp(&numeric_id(&left.id)))
}
