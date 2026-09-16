# 2. Two-step admin transfer

## Status

Accepted

## Context

The original `set_admin(new_admin)` rotated the admin address in a single
call: the current admin calls it, `require_auth()`s, and the stored admin
address is overwritten immediately. Nothing on-chain verifies that
`new_admin` is correct, reachable, or capable of producing a valid
signature before the swap happens.

The admin role is uniquely unforgiving of this: per `DECISIONS.md`, the
admin key can call `upgrade()` (arbitrary WASM replacement), `pause()`, and
`set_relay_signer()`. If a single-step rotation is called with a typo'd
address, an address whose multisig/DAO signer setup hasn't actually been
finalized yet, or any address that turns out not to be controllable, the
contract has no recovery path — `set_admin()` itself requires the
*current* admin's auth, which by definition no longer exists once the
(bad) new admin has been written. The only way out would be a fresh
deployment, discarding contract identity and all persistent `Transaction`
history.

## Decision

Replace the single-step rotation with a two-step propose/accept handshake:

1. `propose_admin(new_admin)` — callable only by the current admin. Stores
   `new_admin` as `StorageKey::PendingAdmin`. Emits `admin_prop`
   (`AdminTransferProposed`). Does **not** change who the active admin is.
2. `accept_admin()` — callable by anyone, but requires `new_admin.require_auth()`
   to succeed, so only the actual proposed address can complete it. On
   success, overwrites `StorageKey::Admin`, clears `PendingAdmin`, and
   emits `admin_upd` (`AdminUpdated`) — the same event `set_admin()` used to
   emit, so downstream consumers watching for `admin_upd` need no changes.

If `propose_admin()` was never called (or its result was already consumed
by a prior `accept_admin()`), `accept_admin()` fails with
`Error::NoPendingAdmin` rather than silently no-op'ing.

Alternatives considered:

- **Keep `set_admin()`, add propose/accept as an alternative path.** Rejected:
  leaving the unsafe single-step path in place defeats the purpose — anyone
  could still use it and hit the exact lockout scenario this ADR exists to
  prevent.
- **Add a `cancel_admin_transfer()` to let the current admin retract a
  pending proposal.** Not included in this change — `propose_admin()` can
  simply be called again with a corrected address to overwrite the pending
  value, which covers the realistic "I made a typo" case without a third
  entry point. Revisit if a real need for explicit cancellation (distinct
  from overwrite) shows up.

## Consequences

- Rotating the admin now takes two transactions instead of one, and the
  second one must be submitted by the incoming admin's own signer(s) — an
  operational step the deployment/key-ceremony process (`DEPLOYMENT.md`)
  needs to account for.
- `THREAT_MODEL.md`'s admin-key-compromise finding (F3) is unaffected by
  this change — this ADR closes the *accidental lockout* risk, not the
  *compromised-key* risk, which is orthogonal and still mitigated only by
  the multisig/DAO custody requirement in `DECISIONS.md`.
- `EVENTS.md` gains one additive event (`admin_prop`); `admin_upd`'s shape
  is unchanged, so no existing subscriber decoding logic breaks.
