//! Error type for [`crate`] operations.

use std::io;
use std::path::PathBuf;
use thiserror::Error;

/// All failure modes of the timeout-bounded Unix socket operations in this
/// crate.
#[derive(Debug, Error)]
pub enum UdsError {
    /// [`crate::connect`] exceeded its deadline. The connect itself may still
    /// be in flight on the blocking pool (it is abandoned, not cancelled);
    /// the error tells you *which* socket path failed to connect in time.
    #[error("timed out connecting to socket at {path:?}")]
    ConnectTimeout {
        /// The socket path that failed to connect within the deadline.
        path: PathBuf,
    },

    /// The socket path resolved and `connect(2)` ran, but the OS rejected or
    /// failed the connection (e.g. `ENOENT` for a nonexistent path,
    /// `ECONNREFUSED` for a stale socket file, `EACCES` for permissions).
    /// Immediate OS-level failures surface here, *not* as
    /// [`UdsError::ConnectTimeout`] — the timeout variant is reserved for
    /// connects that block past the deadline.
    #[error(transparent)]
    Io(#[from] io::Error),

    /// [`crate::read_with_timeout`] exceeded its deadline before the peer
    /// closed its write side (`read_to_end` waits for EOF).
    #[error("timed out reading from socket")]
    ReadTimeout,

    /// [`crate::write_all_with_timeout`] exceeded its deadline before the
    /// full buffer was flushed to the socket.
    #[error("timed out writing to socket")]
    WriteTimeout,
}
