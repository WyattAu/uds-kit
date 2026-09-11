# uds-kit

[![docs.rs](https://docs.rs/uds-kit/badge.svg)](https://docs.rs/uds-kit)
[![crates.io](https://img.shields.io/crates/v/uds-kit.svg)](https://crates.io/crates/uds-kit)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)

Timeout-aware Unix domain socket plumbing for Tokio.

Two real-world call sites (a Firecracker API client and a ClamAV INSTREAM
scanner) each hand-rolled their own `tokio::time::timeout` wrappers around
`UnixStream` operations — and both were **missing a write timeout**, a gap
that silently left write-side hangs unprotected. This crate centralizes the
pattern:

- [`connect`] — `UnixStream::connect` bounded by a timeout; expiry is a typed
  `UdsError::ConnectTimeout { path }` (unlike a bare elapsed `Elapsed` error,
  you learn *which* socket timed out).
- [`read_with_timeout`] — `read_to_end` bounded by a timeout.
- [`write_all_with_timeout`] — `write_all` bounded by a timeout (the gap the
  hand-rolled wrappers never covered).

**Transport only.** Callers keep their own protocol logic — request
formatting, HTTP framing, ClamAV chunked INSTREAM envelopes, length-prefixed
messages. This crate does exactly one thing: bound the three primitive socket
operations with deadlines and surface expiry as typed errors.

Unix-only. The crate root is `#![cfg(unix)]`, so on Windows `cargo check`
compiles an empty crate rather than erroring.

## Example

```rust
use std::time::Duration;
use uds_kit::{connect, write_all_with_timeout, read_with_timeout};

# async fn demo() -> Result<(), uds_kit::UdsError> {
let mut stream = connect("/var/run/clamav/clamd.sock", Duration::from_secs(5)).await?;

write_all_with_timeout(&mut stream, b"zPING\0", Duration::from_secs(5)).await?;

let mut response = Vec::new();
read_with_timeout(&mut stream, &mut response, Duration::from_secs(5)).await?;
# Ok(())
# }
```

## License

MIT OR Apache-2.0
