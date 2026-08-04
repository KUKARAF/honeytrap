use super::marker::RESERVED_DOMAINS;
use crate::seed::RngHandle;

const LOCAL_PARTS: &[&str] = &["admin", "root", "noreply", "support", "db"];

/// A plausible service-account-style email under a reserved domain.
pub fn email(rng: &RngHandle) -> String {
    let local = LOCAL_PARTS[rng.gen_range_u32(LOCAL_PARTS.len() as u32) as usize];
    let suffix = super::secrets::hex(rng, 4);
    let domain = RESERVED_DOMAINS[rng.gen_range_u32(RESERVED_DOMAINS.len() as u32) as usize];
    let domain = if domain == "invalid" {
        "mail.invalid".to_string()
    } else {
        domain.to_string()
    };
    format!("{local}-{suffix}@{domain}")
}

/// A random RFC 4122 version-4-formatted UUID. Bytes come from the seeded
/// RNG (not `uuid::Uuid::new_v4`, which uses non-deterministic system
/// randomness) — the `uuid` crate is used only for correct 8-4-4-4-12
/// formatting and version/variant bit placement.
pub fn uuid(rng: &RngHandle) -> String {
    let mut bytes: [u8; 16] = rng.next_bytes(16).try_into().unwrap();
    bytes[6] = (bytes[6] & 0x0f) | 0x40; // version 4
    bytes[8] = (bytes[8] & 0x3f) | 0x80; // variant 10
    uuid::Builder::from_bytes(bytes).into_uuid().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn email_is_reserved_domain() {
        let rng = RngHandle::new([11u8; 32]);
        for _ in 0..50 {
            let e = email(&rng);
            assert!(
                e.ends_with("mail.invalid") || e.ends_with("example.com"),
                "{e}"
            );
        }
    }

    #[test]
    fn uuid_is_version_4() {
        let rng = RngHandle::new([12u8; 32]);
        let u = uuid(&rng);
        let parsed = uuid::Uuid::parse_str(&u).unwrap();
        assert_eq!(parsed.get_version_num(), 4);
    }
}
