# Decisions

Short-form record of standing design decisions that the code and docs in
this repo assume are true. For decisions with fuller rationale and
alternatives considered, see `docs/adr/`. This file is the quick-reference
index; `docs/adr/` is where the "why" lives for anything non-obvious.

## Admin key must be multisig or DAO-held

`initialize()`'s `admin` parameter, and every subsequent `admin` produced by
`accept_admin()` (see the two-step transfer entry below), **must** be a
multisig (≥3-of-5 threshold) or DAO-controlled Stellar account. Never a
single signer — not even on testnet.

**Why:** the admin address can call `propose_upgrade()`/`execute_upgrade()`,
which replaces the contract's WASM outright (subject to the timelock in
`docs/adr/0003-upgrade-timelock.md`). A compromised single-signer admin key
doesn't just let an attacker rotate roles or pause the contract; it lets them deploy
arbitrary code with full access to every stored `Transaction` and to the
`relay_signer` role. The contract has no on-chain way to enforce that an
`Address` is actually a multisig/DAO account (Soroban doesn't expose signer
thresholds to contract code), so this is an operational requirement on
whoever calls `initialize()` and `propose_admin()`/`accept_admin()`, not
something `require_auth()` can verify for you. See
`docs/adr/0001-relay-signer-trust-model.md` for how
this interacts with the relay signer's (deliberately weaker) trust model,
and `DEPLOYMENT.md` for what the key ceremony needs to look like in
practice.

**Where enforced (operationally, not on-chain):** deployment checklist in
`DEPLOYMENT.md`; code review checklist item 1 in `CLAUDE.md`.

## Admin rotation is a two-step propose/accept handshake

`propose_admin(new_admin)` (current admin) followed by `accept_admin()`
(called by `new_admin` itself) replaces what used to be a single-step
`set_admin(new_admin)`. Nothing changes until `accept_admin()` succeeds.

**Why:** a single-step rotation trusts that `new_admin` is correct and
reachable at the moment it's set, with no way to verify that before the
swap. A typo'd address, or one for which no valid signing setup actually
exists yet, would permanently lock out admin control — there's no
`require_auth()` on an address that can't produce a signature, and no
"undo" once the old admin has been overwritten. Requiring the *proposed*
address to call `accept_admin()` proves it can actually sign before the
handshake completes, at the cost of one extra transaction. See
`docs/adr/0002-two-step-admin-transfer.md`.

**Where enforced:** `admin::propose_admin`/`admin::accept_admin`
(`src/admin.rs`); covered by
`test_propose_and_accept_admin_transfer`,
`test_accept_admin_requires_proposed_admin_auth`, and
`test_accept_admin_fails_without_pending_admin` in `src/test.rs`.

## Upgrades are timelocked: propose now, execute after a minimum delay

`propose_upgrade(new_wasm_hash, expected_schema_version)` (admin) followed
by `execute_upgrade()` (admin, no earlier than
`storage::UPGRADE_TIMELOCK_LEDGERS` — ~48h — after the proposal) replaces
what used to be a single-step `upgrade(new_wasm_hash, expected_schema_version)`.

**Why:** the original single-step `upgrade()` took effect in the same
transaction that proposed it — a compromised admin key (or signers
colluding beyond the multisig threshold) could swap in malicious code with
zero warning. A mandatory delay between proposal and execution gives
anyone watching the chain (subscribers to the `upgrade_prop` event, signers
on the admin multisig who didn't actually sign the proposal, off-chain
monitoring) a window to notice and react before the swap takes effect. This
does not require a new trust role or a threshold signature scheme — it only
requires that someone is watching. See
`docs/adr/0003-upgrade-timelock.md` for the alternatives considered
(a guardian/veto role, a configurable delay) and why they were rejected.

**Where enforced:** `admin::propose_upgrade`/`admin::execute_upgrade`
(`src/admin.rs`); covered by
`test_execute_upgrade_before_timelock_elapses_fails` and
`test_execute_upgrade_after_timelock_bumps_schema_version_and_rejects_replay`
in `src/test.rs`.

## `register_callback` idempotency: two independent guards

Duplicate delivery of the completion callback must never re-run the
Confirmed → Completed transition or re-emit `tx_comp`. Two independent
mechanisms enforce this, and both must be preserved if `register_callback()`
or `storage::check_and_mark_callback_seen` is touched:

1. **Temporary-storage fence** (`StorageKey::CallbackSeen`, ~24h TTL):
   catches same-window retries cheaply, without a full read of the
   transaction record.
2. **Durable status check**: if the stored `Transaction` already has
   `status == Completed`, treat the call as a duplicate regardless of
   whether the temporary fence has expired.

**Why two guards instead of one:** temporary storage is not a safe single
source of truth for "did this already happen" — it expires. A relay retry
that arrives after the ~24h fence has expired but is still, in fact, a
duplicate must not re-trigger the transition. The durable status check is
the source of truth; the temporary fence is a cheap fast-path that avoids a
full state read on the common case.

## Free-form string fields are not validated as Stellar strkeys

`sender`, `source_chain`, and `dest_chain` are `String`, validated only for
non-empty and `<= MAX_STRING_LEN` (`validation::validate_string_len`), not
as SEP-23 strkeys.

**Why:** the source chain isn't necessarily Stellar. `sender` is a
source-chain address in whatever format that chain uses (an Ethereum
address, for example), so it can't be strkey-validated. `recipient` doesn't
have this problem because it's the SDK's native `Address` type, which the
host already validates during XDR decoding.

`validation::validate_strkey_ed25519_public_key` (SEP-23 strkey + CRC16
validation) exists in `validation.rs` for the day an entry point *does* take
a Stellar address as free-form text, but has no caller today — see the
comment at its definition.

## Storage layout changes require a fresh deployment

Any change to the `Transaction` struct's field set/types, or to the
`StorageKey` enum's variants, is a breaking change requiring a fresh
contract deployment — `execute_upgrade()`'s WASM hot-swap does not migrate
existing persistent storage to a new layout. Flag this explicitly in any PR
that touches `types.rs`'s `Transaction` or `StorageKey`.

## Dependency vulnerabilities are scanned in CI, not just at review time

A separate `audit` CI job (`rustsec/audit-check`) runs on every push/PR,
independent of `make check`, plus `.github/dependabot.yml` for proactive
`cargo`/`github-actions` update PRs.

**Why:** this repo's own code being correct (`make check`) says nothing
about whether its ~215 transitive dependencies are free of disclosed
vulnerabilities — that can change without this repo's code changing at
all. A separate job keeps that signal distinct from "this PR's code is
wrong." See `docs/adr/0004-dependency-vulnerability-scanning.md`.

**Where enforced:** `.github/workflows/ci.yml`'s `audit` job;
`.github/dependabot.yml`. Findings are tracked in `THREAT_MODEL.md`
with the same open/accepted-risk discipline as any other finding (see F9).
