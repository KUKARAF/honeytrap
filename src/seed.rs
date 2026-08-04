use rand_chacha::ChaCha8Rng;
use rand_core::{RngCore, SeedableRng};
use std::cell::RefCell;

/// Domain-separation / version tag so the derivation can change later without
/// silently colliding with seeds produced by an older binary.
const SEED_DOMAIN_TAG: &[u8] = b"honeytrap-seed-v1";

/// Derive a deterministic 32-byte RNG seed from the client IP, requested Host
/// header, and an operator-supplied salt. Same inputs always produce the same
/// seed; changing any one input produces an unrelated seed.
pub fn derive_seed(client_ip: &str, host: &str, salt: &str) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(SEED_DOMAIN_TAG);
    hasher.update(client_ip.as_bytes());
    hasher.update(b"\0");
    hasher.update(host.to_lowercase().as_bytes());
    hasher.update(b"\0");
    hasher.update(salt.as_bytes());
    *hasher.finalize().as_bytes()
}

/// Wraps a seeded ChaCha8Rng behind interior mutability so it can be shared
/// (via `Rc`) with template global functions during a single, synchronous
/// render call. Never shared across threads or across requests.
pub struct RngHandle(RefCell<ChaCha8Rng>);

impl RngHandle {
    pub fn new(seed: [u8; 32]) -> Self {
        Self(RefCell::new(ChaCha8Rng::from_seed(seed)))
    }

    pub fn next_bytes(&self, n: usize) -> Vec<u8> {
        let mut buf = vec![0u8; n];
        self.0.borrow_mut().fill_bytes(&mut buf);
        buf
    }

    pub fn gen_range_u32(&self, upper_exclusive: u32) -> u32 {
        // Simple modulo reduction is fine here: this drives cosmetic fake-data
        // choices (which vocabulary word, which byte), not anything
        // security-sensitive, so the (negligible) modulo bias is irrelevant.
        self.0.borrow_mut().next_u32() % upper_exclusive
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_inputs_same_seed() {
        assert_eq!(
            derive_seed("1.2.3.4", "example.com", "s"),
            derive_seed("1.2.3.4", "example.com", "s")
        );
    }

    #[test]
    fn different_host_different_seed() {
        assert_ne!(
            derive_seed("1.2.3.4", "a.example.com", "s"),
            derive_seed("1.2.3.4", "b.example.com", "s")
        );
    }

    #[test]
    fn different_ip_different_seed() {
        assert_ne!(
            derive_seed("1.1.1.1", "example.com", "s"),
            derive_seed("2.2.2.2", "example.com", "s")
        );
    }

    #[test]
    fn different_salt_different_seed() {
        assert_ne!(
            derive_seed("1.2.3.4", "example.com", "s1"),
            derive_seed("1.2.3.4", "example.com", "s2")
        );
    }

    #[test]
    fn host_case_insensitive() {
        assert_eq!(
            derive_seed("1.2.3.4", "Example.COM", "s"),
            derive_seed("1.2.3.4", "example.com", "s")
        );
    }
}
