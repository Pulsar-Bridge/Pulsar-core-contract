# Changelog

This file tracks notable changes to `pulsar-core-contract`. The **Event
schema** subsection under each release is kept current independently of
general code notes — per `CLAUDE.md`, don't let a general commit bury an
event-schema change. Any entry there should correspond to a version note in
`EVENTS.md` §5.

## [Unreleased]

### Added
- Initial contract scaffold: types, storage, validation, admin, events, and
  entry points for the full deposit lifecycle (`register_transaction` ->
  `confirm_transaction` -> `register_callback`, with `fail_transaction` /
  `refund_transaction` as alternate terminal paths from `Pending` or
  `Confirmed`).
- Admin entry points: `pause`/`unpause`, `set_admin`, `set_relay_signer`,
  `upgrade` (schema-version-guarded WASM hot-swap).
- 25-test suite covering happy paths, auth failures, invalid input,
  idempotency, state-machine guards, pause/upgrade, an EVENTS.md-conformance
  check, and the SEP-23 strkey validator against `stellar-strkey`'s own test
  vectors.
- `EVENTS.md`, `DECISIONS.md`, `THREAT_MODEL.md`, `DEPLOYMENT.md`, and
  `docs/adr/0001-relay-signer-trust-model.md`.
- `Makefile` with a `check` target (`fmt` -> `wasm build` -> `clippy` ->
  `test`) matching the bar `CLAUDE.md` and `CONTRIBUTING.md` describe.
- `.github/workflows/ci.yml` running `make check` on every push and pull
  request — `CLAUDE.md` and `CONTRIBUTING.md` both described this as the bar
  "CI enforces," but no CI was actually configured until now.

### Fixed
- `rust-toolchain.toml` targeted `wasm32-unknown-unknown`, which current
  Rust (1.82+) enables reference-types/multi-value on by default for — both
  unsupported by the Soroban environment. Retargeted to `wasm32v1-none`,
  the target the Soroban host actually supports on modern Rust.
- Migrated event publishing from the deprecated `env.events().publish(...)`
  tuple API to `#[contractevent]` structs (`src/events.rs`). Payloads are
  now self-describing `Symbol -> Val` maps keyed by field name instead of
  positional tuples — a meaningful improvement for a schema meant to be a
  stable cross-repo contract, since downstream decoders no longer depend on
  field order.
- `Cargo.toml`'s `repository` field and `CLAUDE.md`'s opening line both
  pointed at `github.com/Synapse-bridgez/synapse-core-contracts`, which
  doesn't match this repo's actual remote
  (`github.com/Pulsar-Bridge/Pulsar-core-contract`) — stale metadata from
  before an org/repo rename. Corrected both to the real remote.
- Added unit tests for `validate_strkey_ed25519_public_key` (SEP-23 strkey +
  CRC16 check) against `stellar-strkey`'s own test vectors. This function
  had no caller anywhere in the contract and so was entirely unverified;
  it's now proven correct (valid keys accepted; wrong length, wrong version
  byte, and corrupted checksum all rejected) before any future entry point
  comes to depend on it.
- `DEPLOYMENT.md` referenced the deprecated `stellar contract install`;
  corrected to `stellar contract upload`.
- No test previously asserted on an emitted event's actual topics/payload —
  every event helper in `src/events.rs` was exercised only indirectly
  through entry points returning `Ok`. Added
  `test_register_transaction_emits_tx_reg_event`, which compares the real
  XDR the host emits for `register_transaction` against `EVENTS.md`'s
  documented `tx_reg` topics and field set, catching drift between the two
  that nothing else in the suite would.

### Event schema
- `EVENTS.md` schema version: **1**. Ten events defined: `init`, `tx_reg`,
  `tx_conf`, `tx_comp`, `tx_fail`, `tx_refund`, `pause`, `admin_upd`,
  `relay_upd`, `upgrade`. This is the first published version — no
  subscribers yet, so no advance-notice obligation applied to this release.
