// Test code: unwrap/expect are the idiomatic way to assert assumptions.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Integration tests for [`uds_kit::connect`].

#![cfg(unix)]

use std::time::Duration;

use tokio::net::UnixListener;
use uds_kit::{connect, UdsError};

const SHORT: Duration = Duration::from_millis(500);

#[tokio::test]
async fn connect_success_against_bound_listener() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("live.sock");

    let _listener = UnixListener::bind(&path).expect("bind");

    let stream = connect(&path, SHORT).await.expect("connect should succeed");
    assert_eq!(
        stream.peer_addr().expect("peer_addr").as_pathname(),
        Some(path.as_path()),
        "should be connected to the bound socket path"
    );
}

#[tokio::test]
async fn connect_nonexistent_path_is_immediate_io_error() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("does-not-exist.sock");

    let started = std::time::Instant::now();
    let err = connect(&path, SHORT).await.expect_err("must fail");
    let elapsed = started.elapsed();

    // Immediate ENOENT surfaces as Io, not ConnectTimeout: the OS rejected
    // the connect outright, no deadline was consumed.
    match err {
        UdsError::Io(e) => assert_eq!(e.kind(), std::io::ErrorKind::NotFound),
        other => panic!("expected Io(NotFound), got {other:?}"),
    }
    assert!(
        elapsed < SHORT,
        "immediate OS error must not consume the full deadline (took {elapsed:?})"
    );
}

// Notes on `ConnectTimeout` coverage: the elapsed→`ConnectTimeout { path }`
// mapping is unit-tested in `src/lib.rs` (`bounded_maps_elapsed_to_caller_error`)
// against a `pending()` future, because on modern Linux an AF_UNIX connect
// never pends — the nonblocking fast path completes instantly against a live
// listener and fails fast (`ENOENT`/`ECONNREFUSED`/`EAGAIN`) otherwise, so no
// filesystem socket can stall a connect deterministically. On
// kernels/filesystems where connect(2) does block (older kernels,
// NFS-hosted sockets), that same mapping is what fires.
