# honeytrap

A small Rust service that serves fabricated, plausible-looking credentials at
common secret-leak paths (`/.env`, `/.aws/credentials`, `/.git/config`, ...).
Nothing real is ever hosted at these paths — every response is generated on
the fly. The goal is to make credential-harvesting scanners bank fake
credentials and waste effort trying to validate them, at effectively zero
cost or risk to you.

It's designed to sit behind [Caddy](https://caddyserver.com/) (or any reverse
proxy) in a homelab: Caddy forwards probe paths to `honeytrap`, everything
else is handled normally by your real services.

## How it works

- **Template-driven.** Every artifact type (`.env`, `wp-config.php`, ...) is a
  plain text file in `--templates ./templates` using
  [minijinja](https://docs.rs/minijinja) syntax. Adding a new artifact type
  means dropping in a new `<path-as-filename>.tmpl` file — no recompile, no
  code change, no routing config.
- **Deterministic per (client IP, Host).** The RNG seed is
  `blake3(client_ip, Host header, --secret salt)`. The same scanner
  re-requesting the same target host gets byte-identical content every time;
  the same scanner hitting a *different* Host looks like an entirely
  unrelated system. This matters: a honeypot whose "leaked" credentials
  shift between requests is instantly recognizable as fake.
- **Bounded.** Every template is rendered and size-checked against
  `--max-bytes` (default 8192) **at startup** — an oversized template is a
  loud startup failure, not a runtime surprise. The point of a tarpit is to
  cost the scanner more than it costs you.

## Usage

```
honeytrap serve --listen 127.0.0.1:8090 --templates ./templates --secret <random-string> [--max-bytes 8192] [--trusted-proxy]
honeytrap gen --path /.env --ip 1.2.3.4 --host example.com --templates ./templates --secret <random-string>
```

`serve` runs the HTTP server. `gen` renders a single response to stdout so
any log line (`client_ip`, `host`, `path`) can be reproduced offline for
debugging — it uses the exact same rendering code path as `serve`.

`--secret` is required and must stay the same across restarts, or the
determinism guarantee above breaks.

`--trusted-proxy` makes the server trust `X-Forwarded-For` (first entry) for
the client IP instead of the raw socket peer. Only enable this when you
actually sit behind a proxy that sets the header — otherwise a scanner could
spoof it to manipulate which "identity" it's seeded as.

## Quickstart: Docker Compose + Caddy

1. Set a persistent secret salt (generate once, keep stable):

   ```
   echo "HONEYTRAP_SECRET=$(openssl rand -hex 32)" > .env
   ```

2. Start the service (binds to loopback only — Caddy on the same host talks
   to it directly):

   ```
   docker compose up -d
   ```

3. Point Caddy's probe paths at it. Example `Caddyfile` snippet:

   ```caddyfile
   your-real-site.example.com {
       # Route known scanner probe paths to the honeypot before anything else.
       @honeytrap {
           path /.env* /.aws/credentials /.git/config /config.json
           path /wp-config.php /docker-compose.yml /.npmrc /secrets.json
       }
       handle @honeytrap {
           reverse_proxy 127.0.0.1:8090 {
               header_up X-Forwarded-For {remote_host}
               header_up Host {host}
           }
       }

       # ... the rest of your real site's config ...
       reverse_proxy 127.0.0.1:3000
   }
   ```

   Caddy sets `X-Forwarded-For` by default on `reverse_proxy`, but it's
   spelled out above for clarity — `honeytrap` only honors it because
   `docker-compose.yaml` passes `--trusted-proxy`.

4. Ship the JSON logs (`docker compose logs -f honeytrap`) to your
   Loki/Grafana stack — see [Log schema](#log-schema) below.

## Why this is safe to run

Every generated value is syntactically valid (matches the shape a real
credential of that type would have) but is either drawn from an
IANA/RFC-reserved namespace that can never be real, or carries an embedded
marker so you can identify it later:

| Function | Scheme |
|---|---|
| `aws_access_key()` | `AKIA[A-Z0-9]{16}` shape, with the literal marker `HTRAP` embedded at a fixed offset in the suffix. Never a real, ever-issued AWS key — an attacker (or automated scanner) calling `sts:GetCallerIdentity` against it gets `InvalidClientTokenId`. |
| `password(n)`, `hex(n)`, `base64(n)` | Random output of exactly `n` characters from a fixed alphabet. No reserved-fake namespace exists for raw secrets; safety comes from the fact these values unlock nothing real. |
| `jwt()` | Structurally valid 3-segment JWT. The decoded payload's `iss` claim is `honeytrap.invalid` (RFC 2606 reserved TLD) — self-documenting as fake even if decoded. Signature bytes are random, never a real HMAC. |
| `private_ip()` | Drawn from RFC 5737 documentation ranges (`192.0.2.0/24`, `198.51.100.0/24`, `203.0.113.0/24`) — IANA-reserved for documentation, never assigned to real infrastructure, indistinguishable from a real internal subnet at a glance. |
| `hostname()`, `email()` | Always under `example.com` or `.invalid` (RFC 2606 reserved domains) — guaranteed to never resolve. |
| `uuid()` | Random RFC 4122 v4-formatted UUID. No real/fake distinction applies; only shape matters. |

To identify honeytrap-sourced content in a captured scanner payload
elsewhere:

```
grep -rE 'HTRAP|\.invalid|example\.com|203\.0\.113\.|198\.51\.100\.|192\.0\.2\.' <payload>
```

More robustly: every served response's full content is hashed (blake3) and
logged, so a captured payload can be matched back to an exact log line
without relying on any marker surviving copy/paste or partial exfiltration.

## Adding a new artifact type

Drop a file named after the request path into the templates directory, with
`/` replaced by `__` and a `.tmpl` extension, e.g. `/backup.sql` →
`templates/backup.sql.tmpl`. Restart the server (or just re-run — startup
validation re-checks everything). Need a path with characters that don't map
cleanly, or want multiple paths served by one template? Add an alias in
`templates/manifest.toml`.

## Log schema

One JSON line per request to stdout (`tracing`, ships to Loki/Grafana):

```json
{"timestamp":"...","level":"INFO","fields":{"event":"served","client_ip":"1.2.3.4","host":"example.com","path":"/.env","template":".env.tmpl","bytes":989,"response_hash":"..."}}
```

`event` is `"served"` for a rendered response or `"miss"` for an unrecognized
path (404).

## Development

```
cargo build --release --target x86_64-unknown-linux-musl   # static binary
cargo test
```

Test coverage: seeding determinism (same `(ip, host)` → identical output,
different host/ip/salt → different output), AWS key shape + marker presence,
startup size-cap enforcement (both rejection of an oversized template and
that shipped templates stay under the default cap), and HTTP routing
(unknown paths 404, `X-Forwarded-For` honored only when `--trusted-proxy` is
set).
