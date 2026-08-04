use super::marker::MARKER_TAG;
use crate::seed::RngHandle;

const SUFFIX_ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
const SUFFIX_LEN: usize = 16;

/// An AWS access key ID shape (`AKIA[A-Z0-9]{16}`) with the greppable marker
/// embedded at a fixed offset in the suffix. Never a real, ever-issued key —
/// see README for why embedding `HTRAP` here is safe.
pub fn access_key(rng: &RngHandle) -> String {
    let marker_start = 5;
    let marker_end = marker_start + MARKER_TAG.len();
    let mut suffix = vec![0u8; SUFFIX_LEN];
    for (i, slot) in suffix.iter_mut().enumerate() {
        if i >= marker_start && i < marker_end {
            *slot = MARKER_TAG.as_bytes()[i - marker_start];
        } else {
            let idx = rng.gen_range_u32(SUFFIX_ALPHABET.len() as u32) as usize;
            *slot = SUFFIX_ALPHABET[idx];
        }
    }
    format!("AKIA{}", String::from_utf8(suffix).unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;
    use regex::Regex;

    #[test]
    fn matches_aws_key_shape_and_contains_marker() {
        let re = Regex::new(r"^AKIA[A-Z0-9]{16}$").unwrap();
        for seed_byte in 0..=255u8 {
            let rng = RngHandle::new([seed_byte; 32]);
            let key = access_key(&rng);
            assert!(re.is_match(&key), "key {key} did not match AWS key shape");
            assert!(key.contains(MARKER_TAG), "key {key} missing marker");
        }
    }
}
