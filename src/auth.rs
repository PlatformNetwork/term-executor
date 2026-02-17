use crate::config::AUTHORIZED_HOTKEY;

pub fn verify_hotkey(hotkey: Option<&str>) -> bool {
    match hotkey {
        Some(k) => k == AUTHORIZED_HOTKEY,
        None => false,
    }
}

pub fn extract_hotkey(headers: &axum::http::HeaderMap) -> Option<String> {
    headers
        .get("X-Hotkey")
        .or_else(|| headers.get("x-hotkey"))
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

#[allow(dead_code)]
pub fn validate_ss58(address: &str) -> bool {
    if address.len() < 2 || !address.starts_with('5') {
        return false;
    }
    bs58::decode(address).into_vec().is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_hotkey_valid() {
        assert!(verify_hotkey(Some(AUTHORIZED_HOTKEY)));
    }

    #[test]
    fn test_verify_hotkey_invalid() {
        assert!(!verify_hotkey(Some("5InvalidHotkey")));
        assert!(!verify_hotkey(None));
    }

    #[test]
    fn test_validate_ss58() {
        assert!(validate_ss58(AUTHORIZED_HOTKEY));
        assert!(!validate_ss58(""));
        assert!(!validate_ss58("not-an-address"));
    }
}
