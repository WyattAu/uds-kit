//! Timeout-aware Unix domain socket plumbing for Tokio.
//!
//! # Motivation
//!
//! Two independent call sites grew the same hand-rolled wrapper layer: a
//! Firecracker MicroVM API client wrapping `tokio::time::timeout` around
//! `UnixStream::connect`, and a ClamAV scanner wrapping it around
//! `read_to_end` for its INSTREAM response read. Both hand-rolled versions
//! had the same two defects: the logic was duplicated (and diverging), and
//! **both were missing a write timeout entirely** — `write_all` ran
//! unbounded, so a wedged peer could hang the writer forever. This crate
//! centralizes the pattern and closes the write-timeout gap.
//!
//! # Transport only
//!
//! Callers keep their own protocol logic. Request formatting, HTTP framing,
//! ClamAV's chunked INSTREAM envelope (4-byte big-endian length prefixes,
//! zero-terminated), custom length-prefixed protocols — none of that lives
//! here. This crate provides exactly three primitives, each bounded by a
//! caller-supplied deadline:
//!
//! | function | wraps | expiry error |
//! |---|---|---|
//! | [`connect`] | `UnixStream::connect` | [`UdsError::ConnectTimeout`] |
//! | [`read_with_timeout`] | `AsyncReadExt::read_to_end` | [`UdsError::ReadTimeout`] |
//! | [`write_all_with_timeout`] | `AsyncWriteExt::write_all` | [`UdsError::WriteTimeout`] |
//!
//! Immediate OS-level connect failures (`ENOENT`, `ECONNREFUSED`,
//! `EACCES`, …) surface as [`UdsError::Io`] — the `ConnectTimeout` variant is
//! reserved for connects that genuinely block past the deadline (e.g. a full
//! accept backlog, a hung filesystem between the process and the socket).
//!
//! # Platform
//!
//! Unix-only by nature. The crate root carries `#![cfg(unix)]`, so on
//! non-Unix targets the crate compiles to *nothing*: `cargo check` succeeds
//! with an empty crate rather than a wall of platform errors. Depend on it
//! unconditionally in cross-platform workspaces; gate your own Unix-specific
//! call sites as usual.
//!
//! # Example
//!
//! ```no_run
//! use std::time::Duration;
//! use uds_kit::{connect, read_with_timeout, write_all_with_timeout, UdsError};
//!
//! # async fn demo() -> Result<(), UdsError> {
//! let mut stream = match connect("/var/run/daemon.sock", Duration::from_secs(5)).await {
//!     Ok(s) => s,
//!     Err(UdsError::ConnectTimeout { path }) => {
//!         eprintln!("daemon socket {path:?} did not accept within 5s");
//!         return Err(UdsError::ConnectTimeout { path });
//!     }
//!     Err(e) => return Err(e),
//! };
//!
//! // Transport only: the caller formats its own protocol frames.
//! write_all_with_timeout(&mut stream, b"zPING\0", Duration::from_secs(5)).await?;
//!
//! let mut response = Vec::new();
//! read_with_timeout(&mut stream, &mut response, Duration::from_secs(5)).await?;
//! # Ok(())
//! # }
//! ```

#![cfg(unix)]
#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod error;

use std::path::Path;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;

pub use error::UdsError;

/// Shared core of all three public operations: run `fut` under `timeout`;
/// on expiry surface `on_elapsed()` (the caller's typed error), otherwise
/// forward the I/O result as [`UdsError`].
async fn bounded<T, F>(
    fut: F,
    timeout: Duration,
    on_elapsed: impl FnOnce() -> UdsError,
) -> Result<T, UdsError>
where
    F: std::future::Future<Output = std::io::Result<T>>,
{
    tokio::time::timeout(timeout, fut)
        .await
        .map_err(|_| on_elapsed())?
        .map_err(UdsError::from)
}

/// Connects to the Unix domain socket at `path`, bounding the connect with
/// `timeout`.
///
/// Wraps [`UnixStream::connect`] in [`tokio::time::timeout`]. If the
/// deadline elapses first, returns [`UdsError::ConnectTimeout`] carrying the
/// attempted `path`. OS-level connect failures (nonexistent path, refused,
/// permission denied) return [`UdsError::Io`] immediately — they do not wait
/// for the deadline.
///
/// The abandoned connect (if any) finishes on Tokio's blocking pool and is
/// dropped there; it does not leak the socket.
pub async fn connect(path: impl AsRef<Path>, timeout: Duration) -> Result<UnixStream, UdsError> {
    bounded(UnixStream::connect(path.as_ref()), timeout, || {
        UdsError::ConnectTimeout {
            path: path.as_ref().to_path_buf(),
        }
    })
    .await
}

/// Reads from `stream` into `buf` until EOF, bounding the read with
/// `timeout`.
///
/// Wraps [`AsyncReadExt::read_to_end`] in [`tokio::time::timeout`]. Because
/// `read_to_end` only returns when the peer closes its write side, the
/// deadline covers the *entire* response: a peer that trickles bytes or
/// never sends EOF fails with [`UdsError::ReadTimeout`]. Returns the number
/// of bytes read (zero if EOF arrived before any data).
///
/// Any bytes read before expiry remain in `buf` (stream semantics); treat
/// the partial content as unspecified on [`UdsError::ReadTimeout`].
pub async fn read_with_timeout(
    stream: &mut UnixStream,
    buf: &mut Vec<u8>,
    timeout: Duration,
) -> Result<usize, UdsError> {
    bounded(stream.read_to_end(buf), timeout, || UdsError::ReadTimeout).await
}

/// Writes all of `buf` to `stream`, bounding the write with `timeout`.
///
/// Wraps [`AsyncWriteExt::write_all`] in [`tokio::time::timeout`]. Expiry —
/// the peer's receive buffer full for the whole deadline, or the socket
/// otherwise wedged — returns [`UdsError::WriteTimeout`]. This is the guard
/// the hand-rolled wrappers at both original call sites were missing.
///
/// On expiry, an arbitrary prefix of `buf` may already have been written to
/// the socket (stream semantics); callers treating the exchange as
/// request/response should consider the connection poisoned, as with any
/// failed write.
pub async fn write_all_with_timeout(
    stream: &mut UnixStream,
    buf: &[u8],
    timeout: Duration,
) -> Result<(), UdsError> {
    bounded(stream.write_all(buf), timeout, || UdsError::WriteTimeout).await
}

#[cfg(test)]
// Test code: unwrap/expect are the idiomatic way to assert outcomes.
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use std::future::pending;
    use std::path::PathBuf;

    /// The expiry mapping is the crate's core logic; tested against a future
    /// that pends forever (deterministic — no race against thread
    /// scheduling). Real connect/read paths that pend are covered by the
    /// integration tests; note modern Linux never lets an AF_UNIX connect
    /// pend (nonblocking fast path succeeds instantly or fails fast), which
    /// is why the mapping is unit-tested here rather than against a
    /// filesystem socket.
    #[tokio::test]
    async fn bounded_maps_elapsed_to_caller_error() {
        let fut = pending::<std::io::Result<UnixStream>>();
        let err = bounded(fut, Duration::ZERO, || UdsError::ConnectTimeout {
            path: PathBuf::from("/nonexistent.sock"),
        })
        .await
        .expect_err("zero deadline must elapse");

        match err {
            UdsError::ConnectTimeout { ref path } => {
                assert_eq!(path, Path::new("/nonexistent.sock"))
            }
            other => panic!("expected ConnectTimeout, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn bounded_forwards_io_errors_unchanged() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "nope");
        let fut: std::future::Ready<std::io::Result<()>> = std::future::ready(Err(io_err));
        let err = bounded(fut, Duration::from_secs(1), || UdsError::WriteTimeout)
            .await
            .expect_err("io error must be forwarded");

        match err {
            UdsError::Io(e) => assert_eq!(e.kind(), std::io::ErrorKind::PermissionDenied),
            other => panic!("expected Io, got {other:?}"),
        }
    }
}
