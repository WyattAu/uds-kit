# PATTERN_ADOPTION: uds-kit

Relationship to the WyattAu pattern canon. Statuses follow the closed
vocabulary: adopted, partial, planned, refused, n/a. Evidence paths are
relative to this repo.

| Pattern | Status | Evidence |
|---|---|---|
| P103 | n/a | Each operation takes an independent caller-supplied deadline (src/lib.rs:82-94, src/lib.rs:107-114). The crate bounds three socket primitives; it never propagates a deadline across call layers, so the canon deadline-propagation pattern does not apply to it |
