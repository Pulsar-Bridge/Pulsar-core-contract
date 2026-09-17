# Threat Model

Living document. Review on a regular cadence, not just when someone asks —
per `CLAUDE.md`'s security review checklist item 4. Each finding is either
**open** (needs code/process work) or **accepted risk** (deliberately not
fixed on-chain, with a rationale and the off-chain mitigation that bounds
it). An accepted-risk finding without a rationale is a bug in this document.

## Actors

- **Admin**: multisig/DAO-held (`DECISIONS.md`). Can pause/unpause, rotate
  admin and relay signer, and upgrade the contract's WASM.
- **Relay signer**: single hot key held by the off-chain `pulsar-core`
  relay. Can drive the transaction lifecycle. See
  `docs/adr/0001-relay-signer-trust-model.md` for the full trust-model
  rationale.
- **Anyone**: read-only queries (`get_transaction`, `get_admin`,
  `get_relay_signer`, `is_paused`, `schema_version`) require no auth.

## Findings

### F1 — Relay signer compromise: bogus deposit registration
**Status:** accepted risk.

A compromised relay signer key can call `register_transaction` with
fabricated `sender`/`amount`/`recipient`/chain values — nothing in this
contract independently verifies a source-chain deposit occurred.
**Mitigation:** this is `pulsar-core`'s job (source-chain observation), not
this contract's; this contract is the on-chain record/lifecycle authority
for deposits the relay reports. Containment on this side is `pause()` +
`set_relay_signer()` by the admin. See ADR 0001.

### F2 — Relay signer compromise: denial of service on in-flight transactions
**Status:** accepted risk.

A compromised relay signer can force legitimate `Pending`/`Confirmed`
transactions to `Failed`/`Refunded`, or simply withhold
`confirm_transaction`/`register_callback` calls to stall them.
**Mitigation:** `pause()` halts further relay-signer action immediately;
`set_relay_signer()` revokes the compromised key. Neither retroactively
undoes an already-forced `Failed`/`Refunded` transition — that's an
inherent property of an irreversible terminal state, not a gap specific to
this contract. Off-chain reconciliation (re-registering under a new
`transaction_id` if the underlying deposit is still valid) is
`pulsar-core`'s recovery path, not this contract's.

### F3 — Admin key compromise
**Status:** open (process-level mitigation exists; partial on-chain
mitigation added for the upgrade path — see below).

A compromised admin key can pause the contract indefinitely, rotate the
relay signer to an attacker-controlled key, or upgrade the contract's WASM
to arbitrary code with full access to every stored `Transaction`.
**Mitigation:** `DECISIONS.md` requires the admin key to be multisig
(≥3-of-5) or DAO-held, which raises the bar from "one leaked key" to "a
threshold of signers colluding or being independently compromised." This
contract has no way to enforce the threshold on-chain (Soroban doesn't
expose signer-threshold introspection to contract code) — it is entirely an
operational requirement on whoever runs `initialize()` and
`propose_admin()`/`accept_admin()`. Additionally, `propose_upgrade()`/
`execute_upgrade()` (`docs/adr/0003-upgrade-timelock.md`) now requires a
~48h delay between proposing and executing a WASM swap, giving anyone
watching the `upgrade_prop` event a window to notice and react before a
malicious upgrade takes effect — this narrows the worst-case impact
(instant, silent code replacement) but does not stop a sufficiently
compromised multisig from eventually pushing the upgrade once the delay
elapses. `pause()`/`set_relay_signer()` rotation are not timelocked and
remain immediately available as the fast-response containment tools.
**Open work:** `DEPLOYMENT.md`'s key-ceremony section needs to stay
operationally real (documented signers, a real signing workflow), not just
a requirement on paper — re-verify this each time custody changes.

### F4 — `execute_upgrade()` schema-version guard bypass
**Status:** closed, re-verify on every `admin.rs`/`execute_upgrade()` change.

`execute_upgrade()` is a compare-and-swap on `expected_schema_version`
(recorded at `propose_upgrade()` time, re-checked at execution): it checks
the guard and bumps the stored version in the same call, so a second call
fails instead of re-running the upgrade — and since the pending proposal is
cleared on success, a second `execute_upgrade()` call fails with
`NoPendingUpgrade` regardless.
Covered by
`test_execute_upgrade_after_timelock_bumps_schema_version_and_rejects_replay`.
Per `CLAUDE.md`'s security checklist item 3, re-verify this invariant by
inspection (not just by the existing test) any time `execute_upgrade()` or
`admin.rs` changes — a refactor that reorders the version-check and
version-write could silently reopen this.

### F5 — Idempotency guard removal on `register_callback` changes
**Status:** closed, re-verify on every `register_callback`/
`check_and_mark_callback_seen` change.

See `DECISIONS.md`'s "two independent guards" entry. Covered by
`test_register_callback_is_idempotent` (within-TTL-window path, via the
temporary-storage fence) and
`test_register_callback_idempotent_after_temp_fence_expires`
(after-the-temporary-fence-expired path, via the durable status check).
The latter previously had no direct test — ledger sequence numbers (which
temporary-storage TTLs are counted in, not wall-clock time) can in fact be
advanced directly in the test harness via
`env.ledger().with_mut(|li| li.sequence_number += N)`, so both guards are
now independently proven rather than one being covered only "in
production."

### F6 — Free-form string fields have no source-chain-format validation
**Status:** accepted risk.

`sender`, `source_chain`, and `dest_chain` are validated only for
non-empty and length (`MAX_STRING_LEN` = 256 bytes) — not as
chain-specific address formats, since the source chain isn't necessarily
Stellar. A malformed or nonsensical `sender`/`source_chain` value that
still satisfies the length check will be stored as-is.
**Mitigation:** this is inherent to bridging a non-Stellar source chain
address space in a general way; format-specific validation, if ever added,
would need to be per-chain and is `pulsar-core`'s concern at the relay
layer (it decides what it reports), not a general-purpose on-chain check
this contract can perform without knowing every supported chain's address
format in advance.

### F7 — Unbounded storage growth via free-form strings
**Status:** accepted risk.

Every `Transaction` stores up to four `String` fields capped at 256 bytes
each (`sender`, `source_chain`, `dest_chain`, plus `reason` on
`fail_transaction`, stored only in the event, not the record). A relay
signer registering many transactions grows persistent storage linearly.
**Mitigation:** bounded by the relay signer being a trusted-but-limited
role (see F1/F2) and by Soroban's storage-rent economics, which charge for
what's written — this is a cost/availability consideration for
`pulsar-core`'s operator, not a distinct on-chain vulnerability beyond what
F1/F2 already cover.

### F8 — Accidental admin lockout via single-step rotation
**Status:** closed by `docs/adr/0002-two-step-admin-transfer.md`.

The original `set_admin(new_admin)` overwrote the admin address in one
call with no verification that `new_admin` was correct or controllable. A
typo'd address, or one whose multisig/DAO signer setup wasn't actually
finished, would have permanently locked out admin control (`upgrade()`,
`pause()`, `set_relay_signer()`) with no on-chain recovery path short of a
fresh deployment.
**Fix:** `set_admin()` was replaced with `propose_admin()` (current admin
proposes) / `accept_admin()` (proposed address confirms via its own
`require_auth()`). The rotation only takes effect once the new address
proves it can sign. Covered by `test_propose_and_accept_admin_transfer`,
`test_accept_admin_requires_proposed_admin_auth`, and
`test_accept_admin_fails_without_pending_admin`.

### F9 — Unmaintained transitive dependency (`paste`)
**Status:** accepted risk, monitored by CI.

`cargo audit` (see CONTRIBUTING.md's "Dependency vulnerability scanning")
flags `paste` v1.0.15 as unmaintained (RUSTSEC-2024-0436) — no CVE, just an
archived-upstream advisory. It is not a direct dependency of this contract;
`cargo tree -i paste` shows it pulled in transitively by `soroban-env-host`
(via the `ark-*` BLS12-381 curve crates) and `wasmi_core`, both dependencies
of `soroban-sdk` itself, not something this repo can drop or replace
independently.
**Mitigation:** `paste` is a proc-macro crate — it expands identifiers at
compile time and contributes no code to the compiled `wasm32v1-none`
artifact this contract actually deploys, so it carries no on-chain runtime
exposure. The CI `audit` job surfaces this warning on every run (without
failing the build, since it's a warning, not a vulnerability) so a future
actual advisory affecting this or any other dependency won't go unnoticed.
Resolving it requires an upstream `soroban-sdk`/`soroban-env-host` release
that moves off `paste`, not a change in this repo.
