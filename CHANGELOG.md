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
- Admin entry points: `pause`/`unpause`, `propose_admin`/`accept_admin`
  (two-step admin transfer), `set_relay_signer`, `upgrade`
  (schema-version-guarded WASM hot-swap).
- 35-test suite covering happy paths, auth failures (an explicit
  auth-failure test for every relay-signer- and admin-gated entry point),
  invalid input, idempotency, state-machine guards, pause/upgrade, the
  two-step admin transfer, an EVENTS.md-conformance check, and the SEP-23
  strkey validator against `stellar-strkey`'s own test vectors.
- `EVENTS.md`, `DECISIONS.md`, `THREAT_MODEL.md`, `DEPLOYMENT.md`, and
  `docs/adr/0001-relay-signer-trust-model.md`.
- `Makefile` with a `check` target (`fmt` -> `wasm build` -> `clippy` ->
  `test`) matching the bar `CLAUDE.md` and `CONTRIBUTING.md` describe.
- `.github/workflows/ci.yml` running `make check` on every push and pull
  request — `CLAUDE.md` and `CONTRIBUTING.md` both described this as the bar
  "CI enforces," but no CI was actually configured until now.

### Changed
- Replaced the single-step `set_admin(new_admin)` with a two-step
  `propose_admin(new_admin)` / `accept_admin()` handshake: the current admin
  proposes, and the rotation only takes effect once the proposed address
  itself calls `accept_admin()`. A typo'd or unreachable `new_admin` under
  the old single-step call would have permanently locked out admin control
  with no recovery path; the new address proving it can sign before the
  swap closes that gap. See `docs/adr/0002-two-step-admin-transfer.md`.

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
- `EVENTS.md` schema version: **1**. Eleven events defined: `init`,
  `tx_reg`, `tx_conf`, `tx_comp`, `tx_fail`, `tx_refund`, `pause`,
  `admin_prop`, `admin_upd`, `relay_upd`, `upgrade`. `admin_prop` was added
  alongside the two-step admin transfer above. This is still the first
  published version — no subscribers yet, so no advance-notice obligation
  applied to this release.
