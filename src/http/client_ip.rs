use axum::http::HeaderMap;
use std::net::{IpAddr, SocketAddr};

/// Resolves the client IP to seed on. Only trusts `X-Forwarded-For` when
/// `trusted_proxy` is set (Caddy sets this header, but honoring it
/// unconditionally would let a scanner spoof its own determinism bucket).
/// Falls back to the raw socket peer address otherwise, or if the header is
/// absent/malformed even when trusted.
pub fn resolve(headers: &HeaderMap, peer: SocketAddr, trusted_proxy: bool) -> String {
    if trusted_proxy {
        if let Some(forwarded) = headers.get("x-forwarded-for").and_then(|v| v.to_str().ok()) {
            if let Some(first) = forwarded.split(',').next() {
                let candidate = first.trim();
                if let Ok(ip) = candidate.parse::<IpAddr>() {
                    return ip.to_string();
                }
                tracing::warn!(
                    value = candidate,
                    "malformed X-Forwarded-For, falling back to socket peer"
                );
            }
        }
    }
    peer.ip().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    fn peer() -> SocketAddr {
        "9.9.9.9:1234".parse().unwrap()
    }

    #[test]
    fn untrusted_ignores_header() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", HeaderValue::from_static("1.2.3.4"));
        assert_eq!(resolve(&headers, peer(), false), "9.9.9.9");
    }

    #[test]
    fn trusted_uses_first_header_entry() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-forwarded-for",
            HeaderValue::from_static("1.2.3.4, 5.6.7.8"),
        );
        assert_eq!(resolve(&headers, peer(), true), "1.2.3.4");
    }

    #[test]
    fn trusted_falls_back_on_malformed_header() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", HeaderValue::from_static("not-an-ip"));
        assert_eq!(resolve(&headers, peer(), true), "9.9.9.9");
    }

    #[test]
    fn trusted_falls_back_when_absent() {
        let headers = HeaderMap::new();
        assert_eq!(resolve(&headers, peer(), true), "9.9.9.9");
    }
}
