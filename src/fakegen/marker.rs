/// Short, greppable tag embedded in generated fake credentials so captured
/// scanner payloads can be traced back to this honeypot. See README for the
/// full list of markers/reserved namespaces used across all generators.
pub const MARKER_TAG: &str = "HTRAP";

/// Reserved-for-documentation domains (RFC 2606) used for all fake
/// hostnames/emails so nothing generated ever resolves to a real host.
pub const RESERVED_DOMAINS: &[&str] = &["example.com", "invalid"];

/// RFC 5737 documentation IPv4 ranges (TEST-NET-1/2/3) — reserved by IANA,
/// never assigned to real infrastructure, but indistinguishable at a glance
/// from a real internal subnet.
pub const RFC5737_RANGES: &[[u8; 3]] = &[[192, 0, 2], [198, 51, 100], [203, 0, 113]];
