use crate::validator_whitelist::ValidatorWhitelist;
use dashmap::DashMap;
use schnorrkel::{PublicKey, Signature};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::warn;

const NONCE_TTL: Duration = Duration::from_secs(300);
const NONCE_REAP_INTERVAL: Duration = Duration::from_secs(60);

const MAX_HOTKEY_LEN: usize = 128;
const MAX_NONCE_LEN: usize = 256;
const MAX_SIGNATURE_LEN: usize = 256;

pub struct NonceStore {
    seen: DashMap<String, Instant>,
}

impl NonceStore {
    pub fn new() -> Self {
        Self {
            seen: DashMap::new(),
        }
    }

    pub fn check_and_insert(&self, nonce: &str) -> bool {
        use dashmap::mapref::entry::Entry;
        match self.seen.entry(nonce.to_string()) {
            Entry::Occupied(_) => false,
            Entry::Vacant(v) => {
                v.insert(Instant::now());
                true
            }
        }
    }

    pub async fn reaper_loop(self: Arc<Self>) {
        let mut interval = tokio::time::interval(NONCE_REAP_INTERVAL);
        loop {
            interval.tick().await;
            let cutoff = Instant::now() - NONCE_TTL;
            self.seen.retain(|_, ts| *ts > cutoff);
        }
    }
}

pub struct AuthHeaders {
    pub hotkey: String,
    pub nonce: String,
    pub signature: String,
}

pub fn extract_auth_headers(headers: &axum::http::HeaderMap) -> Option<AuthHeaders> {
    let hotkey = headers
        .get("X-Hotkey")
        .or_else(|| headers.get("x-hotkey"))
        .and_then(|v| v.to_str().ok())
        .filter(|s| s.len() <= MAX_HOTKEY_LEN)
        .map(|s| s.to_string())?;

    let nonce = headers
        .get("X-Nonce")
        .or_else(|| headers.get("x-nonce"))
        .and_then(|v| v.to_str().ok())
        .filter(|s| s.len() <= MAX_NONCE_LEN)
        .map(|s| s.to_string())?;

    let signature = headers
        .get("X-Signature")
        .or_else(|| headers.get("x-signature"))
        .and_then(|v| v.to_str().ok())
        .filter(|s| s.len() <= MAX_SIGNATURE_LEN)
        .map(|s| s.to_string())?;

    Some(AuthHeaders {
        hotkey,
        nonce,
        signature,
    })
}

pub fn verify_request(
    auth: &AuthHeaders,
    nonce_store: &NonceStore,
    whitelist: &ValidatorWhitelist,
) -> Result<(), AuthError> {
    if !whitelist.is_whitelisted(&auth.hotkey) {
        return Err(AuthError::UnauthorizedHotkey);
    }

    if !validate_ss58(&auth.hotkey) {
        return Err(AuthError::InvalidHotkey);
    }

    let message = format!("{}{}", auth.hotkey, auth.nonce);
    if !verify_sr25519_signature(&auth.hotkey, &message, &auth.signature) {
        return Err(AuthError::InvalidSignature);
    }

    if !nonce_store.check_and_insert(&auth.nonce) {
        return Err(AuthError::NonceReused);
    }

    Ok(())
}

#[derive(Debug)]
pub enum AuthError {
    UnauthorizedHotkey,
    InvalidHotkey,
    NonceReused,
    InvalidSignature,
}

impl AuthError {
    pub fn message(&self) -> &'static str {
        match self {
            AuthError::UnauthorizedHotkey => "Hotkey is not authorized",
            AuthError::InvalidHotkey => "Invalid SS58 hotkey format",
            AuthError::NonceReused => "Nonce has already been used",
            AuthError::InvalidSignature => "Signature verification failed",
        }
    }

    pub fn code(&self) -> &'static str {
        match self {
            AuthError::UnauthorizedHotkey => "unauthorized_hotkey",
            AuthError::InvalidHotkey => "invalid_hotkey",
            AuthError::NonceReused => "nonce_reused",
            AuthError::InvalidSignature => "invalid_signature",
        }
    }
}

fn verify_sr25519_signature(ss58_hotkey: &str, message: &str, signature_hex: &str) -> bool {
    let pub_bytes = match ss58_to_public_key_bytes(ss58_hotkey) {
        Some(b) => b,
        None => {
            warn!("Failed to decode SS58 address");
            return false;
        }
    };

    let public_key = match PublicKey::from_bytes(&pub_bytes) {
        Ok(pk) => pk,
        Err(_) => {
            warn!("Failed to parse sr25519 public key");
            return false;
        }
    };

    let sig_bytes = match hex::decode(signature_hex.strip_prefix("0x").unwrap_or(signature_hex)) {
        Ok(b) if b.len() == 64 => b,
        _ => {
            warn!("Invalid signature hex");
            return false;
        }
    };

    let signature = match Signature::from_bytes(&sig_bytes) {
        Ok(s) => s,
        Err(_) => {
            warn!("Failed to parse sr25519 signature");
            return false;
        }
    };

    let context = schnorrkel::signing_context(b"substrate");
    public_key
        .verify(context.bytes(message.as_bytes()), &signature)
        .is_ok()
}

fn ss58_checksum(data: &[u8]) -> [u8; 2] {
    use blake2::{digest::consts::U64, Blake2b, Digest};
    let mut hasher = Blake2b::<U64>::new();
    hasher.update(b"SS58PRE");
    hasher.update(data);
    let result = hasher.finalize();
    [result[0], result[1]]
}

fn ss58_to_public_key_bytes(address: &str) -> Option<[u8; 32]> {
    let decoded = bs58::decode(address).into_vec().ok()?;
    // SS58 format: [prefix(1-2 bytes)][public_key(32 bytes)][checksum(2 bytes)]
    // For substrate generic (prefix 42), total = 35 bytes (1 + 32 + 2)
    let (prefix_len, key_start) = if decoded.len() == 35 {
        (1, 1)
    } else if decoded.len() == 36 {
        (2, 2)
    } else {
        return None;
    };

    let payload = &decoded[..prefix_len + 32];
    let expected_checksum = &decoded[prefix_len + 32..];
    let actual_checksum = ss58_checksum(payload);

    if expected_checksum != actual_checksum {
        return None;
    }

    let mut key = [0u8; 32];
    key.copy_from_slice(&decoded[key_start..key_start + 32]);
    Some(key)
}

pub fn validate_ss58(address: &str) -> bool {
    if address.len() < 2 || !address.starts_with('5') {
        return false;
    }
    ss58_to_public_key_bytes(address).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_SS58: &str = "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY";

    #[test]
    fn test_validate_ss58_valid() {
        assert!(validate_ss58(TEST_SS58));
    }

    #[test]
    fn test_validate_ss58_invalid() {
        assert!(!validate_ss58(""));
        assert!(!validate_ss58("not-an-address"));
        assert!(!validate_ss58("1234"));
    }

    #[test]
    fn test_ss58_to_public_key_bytes() {
        let bytes = ss58_to_public_key_bytes(TEST_SS58);
        assert!(bytes.is_some());
        assert_eq!(bytes.unwrap().len(), 32);
    }

    #[test]
    fn test_nonce_store_accepts_first_rejects_replay() {
        let store = NonceStore::new();
        assert!(store.check_and_insert("nonce-1"));
        assert!(!store.check_and_insert("nonce-1"));
        assert!(store.check_and_insert("nonce-2"));
    }

    #[test]
    fn test_extract_auth_headers_present() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert("X-Hotkey", TEST_SS58.parse().unwrap());
        headers.insert("X-Nonce", "test-nonce-123".parse().unwrap());
        headers.insert("X-Signature", "0xdeadbeef".parse().unwrap());
        let auth = extract_auth_headers(&headers);
        assert!(auth.is_some());
        let auth = auth.unwrap();
        assert_eq!(auth.hotkey, TEST_SS58);
        assert_eq!(auth.nonce, "test-nonce-123");
        assert_eq!(auth.signature, "0xdeadbeef");
    }

    #[test]
    fn test_extract_auth_headers_missing() {
        let headers = axum::http::HeaderMap::new();
        assert!(extract_auth_headers(&headers).is_none());
    }

    #[test]
    fn test_verify_request_non_whitelisted() {
        let store = NonceStore::new();
        let wl = ValidatorWhitelist::new();
        let auth = AuthHeaders {
            hotkey: "5InvalidHotkey".to_string(),
            nonce: "nonce-1".to_string(),
            signature: "0x00".to_string(),
        };
        let err = verify_request(&auth, &store, &wl).unwrap_err();
        assert!(matches!(err, AuthError::UnauthorizedHotkey));
    }

    #[test]
    fn test_nonce_not_burned_on_invalid_signature() {
        let store = NonceStore::new();
        let wl = ValidatorWhitelist::new();
        wl.insert_for_test(TEST_SS58);

        let auth = AuthHeaders {
            hotkey: TEST_SS58.to_string(),
            nonce: "nonce-should-survive".to_string(),
            signature: "0x".to_string() + &"00".repeat(64),
        };
        let err = verify_request(&auth, &store, &wl).unwrap_err();
        assert!(matches!(err, AuthError::InvalidSignature));

        assert!(
            store.check_and_insert("nonce-should-survive"),
            "Nonce must not be consumed when signature verification fails"
        );
    }

    #[test]
    fn test_extract_auth_headers_rejects_oversized_nonce() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert("X-Hotkey", TEST_SS58.parse().unwrap());
        let long_nonce = "x".repeat(MAX_NONCE_LEN + 1);
        headers.insert("X-Nonce", long_nonce.parse().unwrap());
        headers.insert("X-Signature", "0xdeadbeef".parse().unwrap());
        assert!(extract_auth_headers(&headers).is_none());
    }

    #[test]
    fn test_extract_auth_headers_rejects_oversized_signature() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert("X-Hotkey", TEST_SS58.parse().unwrap());
        headers.insert("X-Nonce", "test-nonce".parse().unwrap());
        let long_sig = "x".repeat(MAX_SIGNATURE_LEN + 1);
        headers.insert("X-Signature", long_sig.parse().unwrap());
        assert!(extract_auth_headers(&headers).is_none());
    }

    #[test]
    fn test_verify_sr25519_roundtrip() {
        use schnorrkel::{Keypair, MiniSecretKey};

        let mini_key = MiniSecretKey::generate();
        let keypair: Keypair = mini_key.expand_to_keypair(schnorrkel::ExpansionMode::Ed25519);
        let pub_key = keypair.public;

        let mut raw = Vec::with_capacity(35);
        raw.push(42u8);
        raw.extend_from_slice(&pub_key.to_bytes());
        let checksum = ss58_checksum(&raw);
        raw.extend_from_slice(&checksum);
        let ss58 = bs58::encode(&raw).into_string();

        let nonce = "test-nonce-42";
        let message = format!("{}{}", ss58, nonce);

        let context = schnorrkel::signing_context(b"substrate");
        let signature = keypair.sign(context.bytes(message.as_bytes()));
        let sig_hex = hex::encode(signature.to_bytes());

        assert!(verify_sr25519_signature(&ss58, &message, &sig_hex));
        assert!(!verify_sr25519_signature(&ss58, "wrong-message", &sig_hex));
    }

    #[test]
    fn test_ss58_rejects_bad_checksum() {
        let mut decoded = bs58::decode(TEST_SS58).into_vec().unwrap();
        let last = decoded.len() - 1;
        decoded[last] ^= 0xFF;
        let bad_ss58 = bs58::encode(&decoded).into_string();
        assert!(ss58_to_public_key_bytes(&bad_ss58).is_none());
    }
}
