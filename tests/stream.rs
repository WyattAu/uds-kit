// Test code: unwrap/expect are the idiomatic way to assert assumptions.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Integration tests for [`uds_kit::read_with_timeout`] and
//! [`uds_kit::write_all_with_timeout`], using connected socket pairs (no
//! filesystem sockets needed).

#![cfg(unix)]

use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;
use uds_kit::{read_with_timeout, write_all_with_timeout, UdsError};

const SHORT: Duration = Duration::from_millis(500);

#[tokio::test]
async fn read_receives_bytes_until_eof() {
    let (mut a, mut b) = UnixStream::pair().expect("pair");

    a.write_all(b"ping response payload").await.expect("write");
    a.shutdown().await.expect("shutdown sends EOF");

    let mut buf = Vec::new();
    let n = read_with_timeout(&mut b, &mut buf, SHORT)
        .await
        .expect("read should succeed");

    assert_eq!(n, b"ping response payload".len());
    assert_eq!(buf, b"ping response payload");
}

#[tokio::test]
async fn read_timeout_when_peer_never_sends_eof() {
    let (a, mut b) = UnixStream::pair().expect("pair");

    // `a` stays open, writes nothing, never shuts down: `read_to_end` on `b`
    // blocks until the deadline elapses.
    let result = read_with_timeout(&mut b, &mut Vec::new(), SHORT).await;

    match result {
        Err(UdsError::ReadTimeout) => {}
        other => panic!("expected ReadTimeout, got {other:?}"),
    }
    drop(a);
}

#[tokio::test]
async fn write_all_is_transparent_on_healthy_socket() {
    let (mut a, mut b) = UnixStream::pair().expect("pair");

    // On a healthy socket the timeout wrapper must be fully transparent:
    // the write succeeds well inside the deadline.
    write_all_with_timeout(&mut a, b"frame-data", SHORT)
        .await
        .expect("write should succeed");

    a.shutdown().await.expect("shutdown sends EOF");

    let mut buf = Vec::new();
    b.read_to_end(&mut buf).await.expect("peer read");
    assert_eq!(buf, b"frame-data");
}

#[tokio::test]
async fn write_large_payload_within_deadline() {
    let (mut a, mut b) = UnixStream::pair().expect("pair");

    let payload = vec![0x5Au8; 1 << 20]; // 1 MiB, exercises partial writes
    let handle = tokio::spawn(async move {
        let mut sink = Vec::new();
        let _ = b.read_to_end(&mut sink).await;
        sink.len()
    });

    write_all_with_timeout(&mut a, &payload, Duration::from_secs(5))
        .await
        .expect("large write should succeed within deadline");

    a.shutdown().await.expect("shutdown");
    assert_eq!(handle.await.expect("task"), payload.len());
    drop(payload);
}
