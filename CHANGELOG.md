# Changelog

All notable changes to this project are documented here. Format: [Keep a
Changelog](https://keepachangelog.com/): versions follow [semver](https://semver.org).

## [1.0.0] - 2026-10-29

### Declared stable

First 1.0. `uds-kit` carries production IPC; this release commits the timeout-aware connect, read, and write helpers as API-stable, Unix-only by nature.

### Guarantees from here on

- Breaking changes to public items require a minor bump plus a `#[deprecated]`
  window of at least one minor release.
- MSRV bumps are minor releases, not patch releases.
- `#![deny(missing_docs)]` and `#![forbid(unsafe_code)]` are enforced.
- Timeout semantics are contractual: the same inputs and timeouts yield the same outcomes on any 1.x release.

### Explicitly not guaranteed

- Additional socket options may arrive in minors behind new methods; existing helper signatures and timeout semantics will not change.

## [Unreleased]

## [0.1.0] - 2026-09-07

### Added
- Initial public release — timeout-bounded UDS connect/read/write for
  Tokio.


### Added
- Initial public release.
