# Threat Model — uds-kit

Reference: STRIDE. Scope: the crate's public API surface (`connect`,
`read_with_timeout`, `write_all_with_timeout`, `UdsError`) as used by a
downstream service talking to local daemons. Trust boundaries: (1) the
socket file's identity and permissions (who can listen on / connect to the
path), (2) the peer process writing bytes into our buffers, (3) the
dependency tree (tokio).

This crate is deliberately **transport-only** — three timeout-bounded
primitives. It holds no secrets, parses no protocol, and its
security-sensitive surface is correspondingly narrow: availability and the
filesystem trust boundary.

## Assets

| ID | Asset | Example |
|----|-------|---------|
| A1 | Availability of the caller's task (no unbounded I/O wait) | Wedged peer hanging `write_all` forever — the original defect this crate fixes |
| A2 | Bounded memory during reads | Trickle-feeding peer growing the read buffer without end |

## STRIDE Analysis

| # | Threat | Category | Surface | Mitigation | Verifying test |
|---|--------|----------|---------|------------|----------------|
| T1 | Peer wedges a write forever (no write timeout) | DoS | `write_all_with_timeout` | The entire exchange is bounded by `tokio::time::timeout`; expiry returns `UdsError::WriteTimeout` and drops the future (socket write-half closed by drop) | `bounded_maps_elapsed_to_caller_error` (core mapping), `write_large_payload_within_deadline` (`tests/stream.rs`) |
| T2 | Peer trickles bytes / never sends EOF (read hang) | DoS | `read_with_timeout` | Deadline covers the *entire* `read_to_end` — a peer that dribbles one byte per interval still hits `ReadTimeout` at the deadline | `read_timeout_when_peer_never_sends_eof` (`tests/stream.rs`), `read_receives_bytes_until_eof` (happy path) |
| T3 | Connect to a hostile or wrong socket file | Spoofing | `connect` | **Not mitigated in-crate** — AF_UNIX addressing *is* the filesystem; who can bind the path is controlled by its mode bits. No `SO_PEERCRED` peer validation is offered. Documented: filesystem permissions are the authentication | `connect_nonexistent_path_is_immediate_io_error`, `connect_success_against_bound_listener` (`tests/connect.rs`) |
| T4 | Unbounded buffer growth within the deadline | DoS | `read_with_timeout` | **Not mitigated** — `read_to_end` grows `buf` to whatever the peer sends before the deadline expires; the timeout bounds *time*, not bytes. Documented residual risk: pair with a response-size contract at the protocol layer | Code review |
| T5 | Deadline mishandling (zero timeout, expired-before-start) | DoS/Correctness | `bounded` core | Zero deadlines elapse deterministically → typed error; genuine OS-level I/O errors (`ENOENT`, `EACCES`) are forwarded unchanged, never swallowed as timeouts | `bounded_maps_elapsed_to_caller_error`, `bounded_forwards_io_errors_unchanged`, `connect_nonexistent_path_is_immediate_io_error` |
| T6 | Poisoned connection treated as reusable after failed write | Tampering | `write_all_with_timeout` | Documented stream semantics: an arbitrary prefix may have been written on expiry; callers are told to treat the connection as poisoned — the crate cannot roll back bytes | Contract documented on `write_all_with_timeout`; no partial-write replay attempted |

## Repudiation

Not applicable — no logging, no identity, no audit surface. Errors carry
paths and I/O kinds only.

## Out of Scope

- Protocol framing and validation (ClamAV INSTREAM, HTTP-over-UDS, etc.):
  explicitly the caller's job; this crate cannot know message boundaries.
- Peer credential checking (`SO_PEERCRED`/`SO_PEERSEC`): not implemented;
  if a deployment needs to authenticate the daemon behind the socket, that
  logic lives in the caller.
- Socket file lifecycle (stale sockets, cleanup, ownership): caller/OS.

## Residual Risks

- **R1 (Medium, accepted):** Unbounded read buffer within the deadline (T4).
  A local peer that blasts data for the full timeout can allocate O(deadline
  × throughput). Mitigation: callers bound expected response size at the
  protocol layer; a byte-capped read API is the natural future hardening.
- **R2 (Low, accepted):** No peer authentication (T3). The crate trusts the
  filesystem: any process able to create/bind the socket path can impersonate
  the daemon. Standard AF_UNIX posture; documented rather than solved.
- **R3 (Low, accepted):** `UdsError::ConnectTimeout` carries the socket path;
  `Debug` renders it. Paths can leak layout hints into logs — trivially
  acceptable for local infrastructure paths.
- **R4 (Low, accepted):** `#![cfg(unix)]` means non-Unix builds are empty —
  a silently missing security boundary if a caller assumes the crate
  enforces something on all platforms. Documented in crate docs.
