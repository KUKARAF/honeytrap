use crate::seed::RngHandle;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};

const PASSWORD_ALPHABET: &[u8] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*()-_=+";

/// A `len`-character random password from a printable, shell-safe alphabet.
pub fn password(rng: &RngHandle, len: usize) -> String {
    (0..len)
        .map(|_| {
            let idx = rng.gen_range_u32(PASSWORD_ALPHABET.len() as u32) as usize;
            PASSWORD_ALPHABET.get(idx).copied().unwrap_or(b'x') as char
        })
        .collect()
}

/// A `len`-character hex string.
pub fn hex(rng: &RngHandle, len: usize) -> String {
    let bytes_needed = len.div_ceil(2);
    let raw = hex::encode(rng.next_bytes(bytes_needed));
    raw[..len.min(raw.len())].to_string()
}

/// A `len`-character base64 string (standard alphabet).
pub fn base64_str(rng: &RngHandle, len: usize) -> String {
    use base64::engine::general_purpose::STANDARD;
    let bytes_needed = (len * 3).div_ceil(4) + 3;
    let raw = STANDARD.encode(rng.next_bytes(bytes_needed));
    raw[..len.min(raw.len())].to_string()
}

/// A structurally valid, three-segment JWT with a `.invalid` issuer claim so
/// even a decoded payload self-documents as fake. The signature segment is
/// random bytes, never a real HMAC (no signing key exists to fake having).
pub fn jwt(rng: &RngHandle) -> String {
    const HEADER_JSON: &str = r#"{"alg":"HS256","typ":"JWT"}"#;
    let header = URL_SAFE_NO_PAD.encode(HEADER_JSON.as_bytes());

    let sub_suffix = hex(rng, 8);
    let iat = 1_700_000_000u64 + rng.gen_range_u32(10_000_000) as u64;
    let exp = iat + 3600;
    let payload_json = format!(
        r#"{{"sub":"htrap-{sub_suffix}","iss":"honeytrap.invalid","iat":{iat},"exp":{exp}}}"#
    );
    let payload = URL_SAFE_NO_PAD.encode(payload_json.as_bytes());

    let signature = URL_SAFE_NO_PAD.encode(rng.next_bytes(32));

    format!("{header}.{payload}.{signature}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn password_has_exact_length() {
        let rng = RngHandle::new([1u8; 32]);
        assert_eq!(password(&rng, 24).len(), 24);
    }

    #[test]
    fn hex_has_exact_length() {
        let rng = RngHandle::new([2u8; 32]);
        assert_eq!(hex(&rng, 33).len(), 33);
        for c in hex(&rng, 32).chars() {
            assert!(c.is_ascii_hexdigit());
        }
    }

    #[test]
    fn base64_has_exact_length() {
        let rng = RngHandle::new([3u8; 32]);
        assert_eq!(base64_str(&rng, 32).len(), 32);
        assert_eq!(base64_str(&rng, 45).len(), 45);
    }

    #[test]
    fn jwt_has_three_segments_and_invalid_issuer() {
        let rng = RngHandle::new([4u8; 32]);
        let token = jwt(&rng);
        let parts: Vec<&str> = token.split('.').collect();
        assert_eq!(parts.len(), 3);
        let payload = String::from_utf8(URL_SAFE_NO_PAD.decode(parts[1]).unwrap()).unwrap();
        assert!(payload.contains("honeytrap.invalid"));
    }
}
