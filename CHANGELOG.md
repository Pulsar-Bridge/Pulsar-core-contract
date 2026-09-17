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
  (two-step admin transfer), `set_relay_signer`, `propose_upgrade`/
  `execute_upgrade` (schema-version-guarded, timelocked WASM hot-swap).
- 86-test suite covering happy paths, auth failures (an explicit
  auth-failure test for every relay-signer- and admin-gated entry point),
  invalid input, idempotency, state-machine guards, pause/upgrade, the
  two-step admin transfer, an EVENTS.md-conformance check, and the SEP-23
  strkey validator against `stellar-strkey`'s own test vectors. Grown from
  the original 38 by closing coverage gaps: a pause-guard test for every
  relay-signer-gated entry point (previously only 2 of 5 had one), the
  full terminal-state transition matrix out of `Completed`/`Failed`/
  `Refunded` (previously 2 of 9 combinations), length-boundary tests for
  every free-form string field including `fail_transaction`'s `reason`
  (previously only `sender`'s empty case), overwrite-behavior tests for
  `propose_admin`/`propose_upgrade`, EVENTS.md-conformance tests for the
  11 events that only had indirect coverage, and NotInitialized tests for
  entry points called before `initialize()`.
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
- Replaced the single-step `upgrade(new_wasm_hash, expected_schema_version)`
  with a timelocked `propose_upgrade(...)` / `execute_upgrade()` pair:
  `execute_upgrade()` fails with `Error::UpgradeTimelockNotElapsed` until
  `storage::UPGRADE_TIMELOCK_LEDGERS` (~48h) have passed since the matching
  `propose_upgrade()` call. The old single-step call took effect instantly,
  giving no one a chance to notice a malicious upgrade from a compromised
  admin key before it landed. See `docs/adr/0003-upgrade-timelock.md` and
  `THREAT_MODEL.md`'s F3.

### Fixed
- `schema_version()` returned a bare `u32` and silently fell back to the
  current build's `SCHEMA_VERSION` constant via `unwrap_or` if the contract
  had never been initialized, instead of erroring like every other
  instance-storage read (`get_admin`, `get_relay_signer`). Both the entry
  point and `storage::get_schema_version` now return `Result<u32, Error>`,
  erroring `NotInitialized` in that case.
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
- `EVENTS.md` schema version: **1**. Twelve events defined: `init`,
  `tx_reg`, `tx_conf`, `tx_comp`, `tx_fail`, `tx_refund`, `pause`,
  `admin_prop`, `admin_upd`, `relay_upd`, `upgrade_prop`, `upgrade`.
  `admin_prop` was added alongside the two-step admin transfer above;
  `upgrade_prop` was added alongside the upgrade timelock (see "Changed"
  below). This is still the first published version — no subscribers yet,
  so no advance-notice obligation applied to this release.
