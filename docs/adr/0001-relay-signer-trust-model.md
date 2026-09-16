# 1. Relay signer trust model

## Status

Accepted

## Context

Every transaction-lifecycle entry point (`register_transaction`,
`confirm_transaction`, `register_callback`, `fail_transaction`,
`refund_transaction`) is gated on a single `relay_signer` address calling
`require_auth()`. That's a materially weaker trust model than the admin
role, which this repo requires to be multisig/DAO-held (see `DECISIONS.md`).
We need to be explicit about why the relay signer doesn't get the same
treatment, and what it means for the rest of the system when it doesn't.

The relay signer represents the off-chain `pulsar-core` relay process: the
thing that watches the source chain, decides a deposit occurred, and drives
the transaction through its lifecycle on-chain. It is a single hot key held
by an automated service, not a human-reviewed multisig — the relay has to
sign and submit transactions unattended and continuously.

## Decision

The relay signer is a single-key role, deliberately not held to the same
multisig/DAO bar as the admin key. Its blast radius is bounded by design:

- It can only drive `Transaction` records through the state machine defined
  in `storage::assert_transition` — it cannot mint funds, cannot change who
  the admin or relay signer is, cannot pause/unpause, and cannot upgrade the
  contract's code.
- Every transition it can cause is visible on-chain via the corresponding
  event (`EVENTS.md`), and the state machine is one-directional and mostly
  terminal — there's no transition that lets a compromised relay signer
  retroactively rewrite history, only push new records into existence or
  move existing ones toward a terminal state.
- If the relay signer key is compromised, the admin's `pause()` and
  `set_relay_signer()` are the containment path: pause halts all relay
  signer actions immediately, and rotating the signer revokes the
  compromised key going forward. This is why `pause()` and
  `set_relay_signer()` exist as distinct admin-only entry points rather than
  folding recovery into `upgrade()`.

What a compromised relay signer key *can* still do, and what bounds it:

- Register bogus `Transaction` records (`register_transaction`) with
  arbitrary `sender`/`amount`/`recipient` values. This is the main residual
  risk: nothing on this contract independently verifies that a registered
  deposit actually happened on the source chain — that verification is
  `pulsar-core`'s job, not this contract's. This contract is the on-chain
  *record and lifecycle authority* for deposits the relay reports, not a
  source-chain light client.
- Force transactions to `Failed`/`Refunded` (denial of service against
  legitimate transactions), or withhold `confirm_transaction`/
  `register_callback` calls to stall them (which pause/unpause and
  relay-signer rotation contain, but don't retroactively undo).

## Consequences

- `pulsar-core`'s off-chain relay-key custody is now this contract's actual
  security boundary for deposit authenticity, not this contract's own
  validation logic. Any hardening of relay-key custody (HSM-backed signing,
  anomaly detection on the relay process, etc.) belongs in `pulsar-core`,
  and should be tracked there — but changes to what the relay signer is
  *authorized to do* on-chain belong here, and should be re-reviewed against
  this ADR.
- `THREAT_MODEL.md` should track relay-signer compromise as a first-class
  scenario, with `pause()` + `set_relay_signer()` as the documented mitigation,
  not as a gap.
- If a future phase needs finer-grained relay authority (for example,
  splitting "can register" from "can confirm/complete"), that's a new ADR,
  not a silent change to this one — the two-tier admin/relay trust split is
  the thing being decided here, and splitting the relay role further changes
  the blast-radius argument above.
