use crate::config::AUTHORIZED_HOTKEY;
use dashmap::DashMap;
use schnorrkel::{PublicKey, Signature};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::warn;

const NONCE_TTL: Duration = Duration::from_secs(300);
const NONCE_REAP_INTERVAL: Duration = Duration::from_secs(60);

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
        if self.seen.contains_key(nonce) {
            return false;
        }
        self.seen.insert(nonce.to_string(), Instant::now());
        true
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
        .map(|s| s.to_string())?;

    let nonce = headers
        .get("X-Nonce")
        .or_else(|| headers.get("x-nonce"))
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())?;

    let signature = headers
        .get("X-Signature")
        .or_else(|| headers.get("x-signature"))
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())?;

    Some(AuthHeaders {
        hotkey,
        nonce,
        signature,
    })
}

pub fn verify_request(auth: &AuthHeaders, nonce_store: &NonceStore) -> Result<(), AuthError> {
    if auth.hotkey != AUTHORIZED_HOTKEY {
        return Err(AuthError::UnauthorizedHotkey);
    }

    if !validate_ss58(&auth.hotkey) {
        return Err(AuthError::InvalidHotkey);
    }

    if !nonce_store.check_and_insert(&auth.nonce) {
        return Err(AuthError::NonceReused);
    }

    let message = format!("{}{}", auth.hotkey, auth.nonce);
    if !verify_sr25519_signature(&auth.hotkey, &message, &auth.signature) {
        return Err(AuthError::InvalidSignature);
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

fn ss58_to_public_key_bytes(address: &str) -> Option<[u8; 32]> {
    let decoded = bs58::decode(address).into_vec().ok()?;
    // SS58 format: [prefix(1-2 bytes)][public_key(32 bytes)][checksum(2 bytes)]
    // For substrate generic (prefix 42), total = 35 bytes (1 + 32 + 2)
    if decoded.len() == 35 {
        let mut key = [0u8; 32];
        key.copy_from_slice(&decoded[1..33]);
        Some(key)
    } else if decoded.len() == 36 {
        // Two-byte prefix
        let mut key = [0u8; 32];
        key.copy_from_slice(&decoded[2..34]);
        Some(key)
    } else {
        None
    }
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

    #[test]
    fn test_validate_ss58_valid() {
        assert!(validate_ss58(AUTHORIZED_HOTKEY));
    }

    #[test]
    fn test_validate_ss58_invalid() {
        assert!(!validate_ss58(""));
        assert!(!validate_ss58("not-an-address"));
        assert!(!validate_ss58("1234"));
    }

    #[test]
    fn test_ss58_to_public_key_bytes() {
        let bytes = ss58_to_public_key_bytes(AUTHORIZED_HOTKEY);
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
        headers.insert("X-Hotkey", AUTHORIZED_HOTKEY.parse().unwrap());
        headers.insert("X-Nonce", "test-nonce-123".parse().unwrap());
        headers.insert("X-Signature", "0xdeadbeef".parse().unwrap());
        let auth = extract_auth_headers(&headers);
        assert!(auth.is_some());
        let auth = auth.unwrap();
        assert_eq!(auth.hotkey, AUTHORIZED_HOTKEY);
        assert_eq!(auth.nonce, "test-nonce-123");
        assert_eq!(auth.signature, "0xdeadbeef");
    }

    #[test]
    fn test_extract_auth_headers_missing() {
        let headers = axum::http::HeaderMap::new();
        assert!(extract_auth_headers(&headers).is_none());
    }

    #[test]
    fn test_verify_request_unauthorized_hotkey() {
        let store = NonceStore::new();
        let auth = AuthHeaders {
            hotkey: "5InvalidHotkey".to_string(),
            nonce: "nonce-1".to_string(),
            signature: "0x00".to_string(),
        };
        let err = verify_request(&auth, &store).unwrap_err();
        assert!(matches!(err, AuthError::UnauthorizedHotkey));
    }

    #[test]
    fn test_verify_sr25519_roundtrip() {
        use schnorrkel::{Keypair, MiniSecretKey};

        let mini_key = MiniSecretKey::generate();
        let keypair: Keypair = mini_key.expand_to_keypair(schnorrkel::ExpansionMode::Ed25519);
        let pub_key = keypair.public;

        // Encode as SS58 (prefix 42 = generic substrate)
        let mut raw = Vec::with_capacity(35);
        raw.push(42u8);
        raw.extend_from_slice(&pub_key.to_bytes());
        let hash = sp_ss58_checksum(&raw);
        raw.extend_from_slice(&hash[..2]);
        let ss58 = bs58::encode(&raw).into_string();

        let nonce = "test-nonce-42";
        let message = format!("{}{}", ss58, nonce);

        let context = schnorrkel::signing_context(b"substrate");
        let signature = keypair.sign(context.bytes(message.as_bytes()));
        let sig_hex = hex::encode(signature.to_bytes());

        assert!(verify_sr25519_signature(&ss58, &message, &sig_hex));
        assert!(!verify_sr25519_signature(&ss58, "wrong-message", &sig_hex));
    }
}

#[cfg(test)]
fn sp_ss58_checksum(data: &[u8]) -> [u8; 64] {
    use sha2::{Digest, Sha512};
    let mut hasher = Sha512::new();
    hasher.update(b"SS58PRE");
    hasher.update(data);
    let result = hasher.finalize();
    let mut out = [0u8; 64];
    out.copy_from_slice(&result);
    out
}
