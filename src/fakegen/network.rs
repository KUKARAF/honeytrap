use super::marker::{RESERVED_DOMAINS, RFC5737_RANGES};
use crate::seed::RngHandle;

const INFRA_WORDS: &[&str] = &[
    "db", "prod", "staging", "internal", "app", "cache", "api", "queue",
];

/// An IPv4 address from an RFC 5737 documentation range (TEST-NET-1/2/3) —
/// looks like a real internal address but is IANA-reserved and never
/// assigned to real infrastructure.
pub fn private_ip(rng: &RngHandle) -> String {
    let range = RFC5737_RANGES[rng.gen_range_u32(RFC5737_RANGES.len() as u32) as usize];
    let last_octet = rng.gen_range_u32(255) + 1;
    format!("{}.{}.{}.{}", range[0], range[1], range[2], last_octet)
}

/// A plausible-looking internal hostname under a reserved domain
/// (`example.com` or `.invalid`, RFC 2606) — guaranteed to never resolve.
pub fn hostname(rng: &RngHandle) -> String {
    let word = INFRA_WORDS[rng.gen_range_u32(INFRA_WORDS.len() as u32) as usize];
    let suffix = super::secrets::hex(rng, 6);
    let domain = RESERVED_DOMAINS[rng.gen_range_u32(RESERVED_DOMAINS.len() as u32) as usize];
    if domain == "invalid" {
        format!("{word}-{suffix}.invalid")
    } else {
        format!("{word}-{suffix}.{domain}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn private_ip_in_reserved_ranges() {
        let rng = RngHandle::new([9u8; 32]);
        for _ in 0..50 {
            let ip = private_ip(&rng);
            let ok = RFC5737_RANGES
                .iter()
                .any(|r| ip.starts_with(&format!("{}.{}.{}.", r[0], r[1], r[2])));
            assert!(ok, "ip {ip} not in an RFC5737 range");
        }
    }

    #[test]
    fn hostname_is_reserved_domain() {
        let rng = RngHandle::new([10u8; 32]);
        for _ in 0..50 {
            let h = hostname(&rng);
            assert!(h.ends_with(".invalid") || h.ends_with("example.com"), "{h}");
        }
    }
}
