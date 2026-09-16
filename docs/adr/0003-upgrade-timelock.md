# 3. Upgrade timelock

## Status

Accepted

## Context

`THREAT_MODEL.md`'s F3 finding says a compromised admin key can "upgrade
the contract's WASM to arbitrary code with full access to every stored
`Transaction`," and that this is open with "no on-chain backstop... possible"
because Soroban doesn't expose multisig signer-threshold introspection to
contract code. That's true for *preventing* a threshold of compromised
signers from eventually authorizing an upgrade — nothing on-chain can
verify the multisig setup itself. But the original `upgrade()` took effect
in the same transaction that proposed it, which meant a compromised key (or
signers colluding beyond the threshold) could swap in malicious code with
zero warning: no one watching the chain would know until it had already
happened.

## Decision

Split `upgrade()` into a propose/execute pair, gated by a minimum delay
rather than an address handshake (contrast with
`docs/adr/0002-two-step-admin-transfer.md`, which is gated by proving
control of an address, not by time):

1. `propose_upgrade(new_wasm_hash, expected_schema_version)` — admin-only.
   Validates `expected_schema_version` against the current stored version
   (fail fast) and records a `PendingUpgrade` with
   `earliest_ledger = current_ledger + UPGRADE_TIMELOCK_LEDGERS`
   (`storage::UPGRADE_TIMELOCK_LEDGERS`, ~48h at ~5s/ledger). Emits
   `upgrade_prop` (`UpgradeProposed`).
2. `execute_upgrade()` — admin-only. Fails with `UpgradeTimelockNotElapsed`
   if the current ledger is still before `earliest_ledger`. Re-checks
   `expected_schema_version` against the current stored version (the same
   compare-and-swap the original `upgrade()` did — see F4), bumps the
   version, clears the pending proposal, and performs the WASM swap in the
   same call. Emits `upgrade` (`ContractUpgraded`), unchanged in shape from
   the original single-step design.

This doesn't require a threshold signature scheme, a guardian/veto role, or
any other actor this contract doesn't already have — it only requires that
*someone* (any off-chain monitoring, any subscriber to `upgrade_prop`, any
signer on the admin multisig who didn't actually sign the proposal) is
watching. The ~48h window is a starting point, not a claim that it's the
right number for every deployment — see Consequences.

Alternatives considered:

- **A separate "guardian" address that can veto/cancel a pending upgrade.**
  Rejected for this change: it introduces a new trust role and its own key-
  management problem, which is a bigger decision than this ADR is trying to
  make. If a real operational need for cancellation emerges, that's better
  as its own ADR discussing who holds the guardian key and what it's
  allowed to do — not folded into a timelock ADR as an afterthought.
- **Configurable timelock duration (an admin-settable parameter).** Rejected:
  a compromised admin key could set the delay to zero, which defeats the
  purpose. A fixed constant means changing it at all requires a
  contract upgrade (i.e., going through this exact timelock).
- **Applying the same timelock to admin/relay-signer rotation.** Out of
  scope here — `propose_admin`/`accept_admin` already has an
  address-based safeguard (ADR 0002) that a time delay doesn't obviously
  improve, and `set_relay_signer` has a much smaller blast radius (see
  `docs/adr/0001-relay-signer-trust-model.md`) that doesn't obviously
  justify the operational cost of a mandatory delay on every rotation.

## Consequences

- Legitimate emergency upgrades (e.g., patching a newly discovered bug) are
  now also delayed by ~48h. This is a deliberate trade-off: `pause()`
  remains available and unaffected by this ADR as the immediate-response
  tool for halting relay-signer activity while an upgrade is prepared and
  waits out its timelock.
- `DEPLOYMENT.md`'s upgrade walkthrough needs to reflect the two-call
  sequence and the wait in between, not describe upgrade as a single
  atomic operation.
- `THREAT_MODEL.md`'s F3 finding is narrowed, not closed: this mitigates
  *silent, instant* code replacement, not a sufficiently-compromised
  multisig eventually pushing an upgrade after the delay elapses. The
  multisig/DAO custody requirement in `DECISIONS.md` remains the actual
  defense against that.
- `EVENTS.md` gains one additive event (`upgrade_prop`); `upgrade`'s shape
  is unchanged, so no existing subscriber decoding logic breaks.
