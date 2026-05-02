use super::{
    operation_file::sha256_digest_bytes, SyncCoreError, SyncCoreErrorKind, SyncCoreResult,
};

#[cfg(test)]
use std::cell::RefCell;

const HMAC_KEY_ENV: &str = "ME_INVENTORY_SYNC_HMAC_KEY";
const HMAC_PREFIX: &str = "hmac-sha256:";
const MIN_HMAC_KEY_BYTES: usize = 16;
const SHA256_BLOCK_BYTES: usize = 64;

#[cfg(test)]
thread_local! {
    static TEST_HMAC_KEY: RefCell<Option<Option<String>>> = RefCell::new(None);
}

#[cfg(test)]
// Used by path-included sync integration tests through sync::test_support.
#[allow(dead_code)]
pub(crate) struct TestHmacKeyGuard {
    previous: Option<Option<String>>,
}

#[cfg(test)]
impl Drop for TestHmacKeyGuard {
    fn drop(&mut self) {
        TEST_HMAC_KEY.with(|slot| {
            *slot.borrow_mut() = self.previous.take();
        });
    }
}

#[cfg(test)]
// Used by path-included sync integration tests through sync::test_support.
#[allow(dead_code)]
pub(crate) fn set_test_hmac_key(raw_key: Option<&str>) -> TestHmacKeyGuard {
    TEST_HMAC_KEY.with(|slot| {
        let previous = slot.replace(Some(raw_key.map(str::to_string)));
        TestHmacKeyGuard { previous }
    })
}

pub(super) fn sign_canonical_bytes(
    domain: &str,
    canonical_bytes: &[u8],
) -> SyncCoreResult<Option<String>> {
    let Some(key) = configured_key()? else {
        return Ok(None);
    };

    Ok(Some(format!(
        "{HMAC_PREFIX}{}",
        hex(&hmac_sha256(&key, domain, canonical_bytes))
    )))
}

pub(super) fn verify_canonical_bytes(
    domain: &str,
    canonical_bytes: &[u8],
    provided_auth: Option<&str>,
) -> Result<(), String> {
    let Some(key) = configured_key().map_err(|error| error.to_string())? else {
        return Ok(());
    };
    let Some(provided_auth) = provided_auth else {
        return Err(format!(
            "Shared sync authentication is required. Set {HMAC_KEY_ENV} on every trusted client or remove it from all clients."
        ));
    };

    let expected = format!(
        "{HMAC_PREFIX}{}",
        hex(&hmac_sha256(&key, domain, canonical_bytes))
    );
    if constant_time_eq(provided_auth.as_bytes(), expected.as_bytes()) {
        Ok(())
    } else {
        Err("Shared sync authentication HMAC does not match.".to_string())
    }
}

fn configured_key() -> SyncCoreResult<Option<Vec<u8>>> {
    #[cfg(test)]
    if let Some(raw_key) = TEST_HMAC_KEY.with(|slot| slot.borrow().clone()) {
        return configured_key_from_raw(raw_key.as_deref());
    }

    let Ok(raw_key) = std::env::var(HMAC_KEY_ENV) else {
        return configured_key_from_raw(None);
    };
    configured_key_from_raw(Some(raw_key.as_str()))
}

fn configured_key_from_raw(raw_key: Option<&str>) -> SyncCoreResult<Option<Vec<u8>>> {
    let Some(raw_key) = raw_key else {
        return Ok(None);
    };
    let key = raw_key.trim();
    if key.is_empty() {
        return Ok(None);
    }
    if key.len() < MIN_HMAC_KEY_BYTES {
        return Err(SyncCoreError::new(
            SyncCoreErrorKind::InvalidEnvelope,
            format!("{HMAC_KEY_ENV} must be at least {MIN_HMAC_KEY_BYTES} bytes."),
        ));
    }

    Ok(Some(key.as_bytes().to_vec()))
}

fn hmac_sha256(key: &[u8], domain: &str, canonical_bytes: &[u8]) -> [u8; 32] {
    let mut key_block = [0u8; SHA256_BLOCK_BYTES];
    if key.len() > SHA256_BLOCK_BYTES {
        key_block[..32].copy_from_slice(&sha256_digest_bytes(key));
    } else {
        key_block[..key.len()].copy_from_slice(key);
    }

    let mut ipad = [0x36u8; SHA256_BLOCK_BYTES];
    let mut opad = [0x5cu8; SHA256_BLOCK_BYTES];
    for index in 0..SHA256_BLOCK_BYTES {
        ipad[index] ^= key_block[index];
        opad[index] ^= key_block[index];
    }

    let mut inner =
        Vec::with_capacity(SHA256_BLOCK_BYTES + domain.len() + 1 + canonical_bytes.len());
    inner.extend_from_slice(&ipad);
    inner.extend_from_slice(domain.as_bytes());
    inner.push(0);
    inner.extend_from_slice(canonical_bytes);
    let inner_digest = sha256_digest_bytes(&inner);

    let mut outer = Vec::with_capacity(SHA256_BLOCK_BYTES + inner_digest.len());
    outer.extend_from_slice(&opad);
    outer.extend_from_slice(&inner_digest);
    sha256_digest_bytes(&outer)
}

fn hex(bytes: &[u8]) -> String {
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        text.push(nibble_to_hex(byte >> 4));
        text.push(nibble_to_hex(byte & 0x0f));
    }
    text
}

fn nibble_to_hex(nibble: u8) -> char {
    match nibble {
        0..=9 => (b'0' + nibble) as char,
        10..=15 => (b'a' + (nibble - 10)) as char,
        _ => unreachable!("nibble is masked to four bits"),
    }
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }

    let mut diff = 0u8;
    for (left, right) in left.iter().zip(right) {
        diff |= left ^ right;
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hmac_changes_when_domain_or_payload_changes() {
        let key = b"0123456789abcdef";
        let first = hmac_sha256(key, "domain-a", br#"{"a":1}"#);
        let different_domain = hmac_sha256(key, "domain-b", br#"{"a":1}"#);
        let different_payload = hmac_sha256(key, "domain-a", br#"{"a":2}"#);

        assert_ne!(first, different_domain);
        assert_ne!(first, different_payload);
    }

    #[test]
    fn constant_time_compare_rejects_length_and_content_mismatches() {
        assert!(constant_time_eq(b"same", b"same"));
        assert!(!constant_time_eq(b"same", b"same!"));
        assert!(!constant_time_eq(b"same", b"xxxx"));
    }
}
