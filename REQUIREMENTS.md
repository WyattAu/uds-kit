# Requirements — uds-kit

Numbered, testable requirements. Every requirement maps to at least one named
test; every security-relevant test cites at least one requirement.

Scope note: `uds-kit` provides exactly three timeout-bounded primitives over
 Tokio `UnixStream` — `connect`, `read_with_timeout` (`read_to_end`), and
`write_all_with_timeout` — plus the `UdsError` taxonomy. It is transport
only: no protocol framing, no retries, no reconnect logic.

## Functional

| ID | Requirement | Priority |
|----|-------------|----------|
| REQ-UD-001 | `connect(path, timeout)` returns a connected `UnixStream` when a listener accepts within the deadline | MUST |
| REQ-UD-002 | `read_with_timeout` reads until EOF and returns the accumulated bytes when the peer shuts down within the deadline | MUST |
| REQ-UD-003 | `write_all_with_timeout` writes the entire payload to a healthy socket and returns byte count | MUST |
| REQ-UD-004 | Each function enforces its own caller-supplied deadline; expiry surfaces as `UdsError::ConnectTimeout` / `ReadTimeout` / `WriteTimeout` respectively — never a hang | MUST |
| REQ-UD-005 | Large payloads complete within the write deadline (no arbitrary internal size cap) | SHOULD |
| REQ-UD-006 | The crate compiles to an empty crate on non-Unix targets (`#![cfg(unix)]`) | SHOULD |

## Security

| ID | Requirement | Priority |
|----|-------------|----------|
| REQ-UD-100 | Immediate OS-level connect failures (`ENOENT`, `ECONNREFUSED`, `EACCES`) surface as `UdsError::Io` without waiting for the deadline — a missing socket cannot be turned into a slow-loris-style stall by the caller's own timeout choice | MUST |
| REQ-UD-101 | A peer that accepts but never responds produces `ReadTimeout`, not unbounded blocking — a wedged peer cannot pin the caller's task indefinitely | MUST |
| REQ-UD-102 | A peer that stops draining produces `WriteTimeout`, not unbounded blocking — closing the write-side gap that motivated the crate | MUST |
| REQ-UD-103 | No panic path exists on peer misbehavior: hostile peers (abrupt EOF, no EOF, dead listener) map to typed errors only | MUST |

## Robustness

| ID | Requirement | Priority |
|----|-------------|----------|
| REQ-UD-200 | Connecting a nonexistent path fails immediately with an Io error | MUST |
| REQ-UD-201 | Errors are `Display`-friendly and typed; no stringly-typed error leakage | SHOULD |

## Traceability Matrix

| Requirement | Test (fn, file) | Property class |
|-------------|-----------------|----------------|
| REQ-UD-001 | `connect_success_against_bound_listener` (`tests/connect.rs`) | integration |
| REQ-UD-002 | `read_receives_bytes_until_eof` (`tests/stream.rs`) | integration |
| REQ-UD-003 | `write_all_is_transparent_on_healthy_socket` (`tests/stream.rs`) | integration |
| REQ-UD-004 | `read_timeout_when_peer_never_sends_eof` (`tests/stream.rs`) | integration |
| REQ-UD-005 | `write_large_payload_within_deadline` (`tests/stream.rs`) | integration |
| REQ-UD-100 | `connect_nonexistent_path_is_immediate_io_error` (`tests/connect.rs`) | integration |
| REQ-UD-101 | `read_timeout_when_peer_never_sends_eof` (`tests/stream.rs`) | integration |
| REQ-UD-102 | `read_timeout_when_peer_never_sends_eof`, `write_all_is_transparent_on_healthy_socket` (`tests/stream.rs`) | integration |
| REQ-UD-103 | `connect_nonexistent_path_is_immediate_io_error`, `read_timeout_when_peer_never_sends_eof` | integration |
| REQ-UD-200 | `connect_nonexistent_path_is_immediate_io_error` (`tests/connect.rs`) | integration |

## Test Count

- 6 integration tests (`tests/connect.rs`, `tests/stream.rs`).
- All-features suite passes with 0 failures; no-default-features suite passes.
