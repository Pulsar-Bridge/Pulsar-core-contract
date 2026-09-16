# Decisions

Short-form record of standing design decisions that the code and docs in
this repo assume are true. For decisions with fuller rationale and
alternatives considered, see `docs/adr/`. This file is the quick-reference
index; `docs/adr/` is where the "why" lives for anything non-obvious.

## Admin key must be multisig or DAO-held

`initialize()`'s `admin` parameter, and every subsequent `admin` produced by
`set_admin()`, **must** be a multisig (≥3-of-5 threshold) or DAO-controlled
Stellar account. Never a single signer — not even on testnet.

**Why:** the admin address can call `upgrade()`, which replaces the
contract's WASM outright. A compromised single-signer admin key doesn't just
let an attacker rotate roles or pause the contract; it lets them deploy
arbitrary code with full access to every stored `Transaction` and to the
`relay_signer` role. The contract has no on-chain way to enforce that an
`Address` is actually a multisig/DAO account (Soroban doesn't expose signer
thresholds to contract code), so this is an operational requirement on
whoever calls `initialize()` and `set_admin()`, not something `require_auth()`
can verify for you. See `docs/adr/0001-relay-signer-trust-model.md` for how
this interacts with the relay signer's (deliberately weaker) trust model,
and `DEPLOYMENT.md` for what the key ceremony needs to look like in
practice.

**Where enforced (operationally, not on-chain):** deployment checklist in
`DEPLOYMENT.md`; code review checklist item 1 in `CLAUDE.md`.

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
contract deployment — `upgrade()`'s WASM hot-swap does not migrate existing
persistent storage to a new layout. Flag this explicitly in any PR that
touches `types.rs`'s `Transaction` or `StorageKey`.
