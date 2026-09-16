# Event Schema

This is the canonical, versioned description of every event
`pulsar-core-contract` emits. It is a cross-repo public API: `pulsar-core`
and `pulsar-web` (and `pulsar-swap` once it exists) subscribe to these topics
and decode these payloads. Treat a change here the way you'd treat a change
to a REST API contract, not an internal implementation detail.

Schema version: **1** (`PulsarCoreContract::schema_version()`, bumped by
`execute_upgrade()`; see §5).

## How to read this document

- **Topics** are the ordered list of `Symbol`/`String` values an event is
  published under. The first topic is always a static event-kind tag; some
  events additionally carry the `transaction_id` as a second, dynamic topic
  so subscribers can filter by transaction without decoding the payload.
- **Payload** is a `Symbol -> Val` map (Soroban's `#[contractevent]` default
  data format), keyed by the field names below. Consumers should decode by
  field name, not by position — the map format doesn't guarantee wire order.
- All events are defined as `#[contractevent]` structs in `src/events.rs`.
  That module is the only place `.publish()` is called from; don't publish
  an event from anywhere else.

## Events

### `init` — `Initialized`
Emitted once, by `initialize()`.

| Field | Type | Notes |
|---|---|---|
| `admin` | `Address` | Must be multisig/DAO-held per `DECISIONS.md`. |
| `relay_signer` | `Address` | |
| `schema_version` | `u32` | Always `1` at initialization. |

### `tx_reg` — `TransactionRegistered`
Emitted by `register_transaction()`. Topics: `("tx_reg", transaction_id)`.

| Field | Type | Notes |
|---|---|---|
| `transaction_id` | `String` | Also a topic. Off-chain relay's idempotency key. |
| `sender` | `String` | Source-chain sender address; not a Stellar strkey. |
| `recipient` | `Address` | |
| `amount` | `i128` | Always `> 0` (enforced by `validation::validate_amount`). |
| `source_chain` | `String` | |
| `dest_chain` | `String` | |
| `created_at` | `u64` | Ledger timestamp. |

### `tx_conf` — `TransactionConfirmed`
Emitted by `confirm_transaction()`. Topics: `("tx_conf", transaction_id)`.

| Field | Type | Notes |
|---|---|---|
| `transaction_id` | `String` | Also a topic. |
| `updated_at` | `u64` | |

### `tx_comp` — `TransactionCompleted`
Emitted by `register_callback()`. Topics: `("tx_comp", transaction_id)`.
**Not** re-emitted on a duplicate callback delivery — see the idempotency
note in `CLAUDE.md`'s "Current state" section and `storage::check_and_mark_callback_seen`.

| Field | Type | Notes |
|---|---|---|
| `transaction_id` | `String` | Also a topic. |
| `updated_at` | `u64` | |

### `tx_fail` — `TransactionFailed`
Emitted by `fail_transaction()`. Topics: `("tx_fail", transaction_id)`.

| Field | Type | Notes |
|---|---|---|
| `transaction_id` | `String` | Also a topic. |
| `reason` | `String` | Free text, length-capped, not otherwise structured. |
| `updated_at` | `u64` | |

### `tx_refund` — `TransactionRefunded`
Emitted by `refund_transaction()`. Topics: `("tx_refund", transaction_id)`.

| Field | Type | Notes |
|---|---|---|
| `transaction_id` | `String` | Also a topic. |
| `updated_at` | `u64` | |

### `pause` — `PauseStateChanged`
Emitted by `pause()` / `unpause()`. Topics: `("pause",)`.

| Field | Type | Notes |
|---|---|---|
| `by` | `Address` | The admin address that made the call. |
| `paused` | `bool` | `true` on `pause()`, `false` on `unpause()`. |

### `admin_prop` — `AdminTransferProposed`
Emitted by `propose_admin()`, step 1 of the two-step admin transfer (see
`docs/adr/0002-two-step-admin-transfer.md`). Topics: `("admin_prop",)`.

| Field | Type | Notes |
|---|---|---|
| `old_admin` | `Address` | Current admin at the time of the proposal. |
| `proposed_admin` | `Address` | Not yet active — only takes effect once this address calls `accept_admin()`. |

### `admin_upd` — `AdminUpdated`
Emitted by `accept_admin()`, step 2 of the two-step admin transfer. Topics: `("admin_upd",)`.

| Field | Type | Notes |
|---|---|---|
| `old_admin` | `Address` | |
| `new_admin` | `Address` | Operationally must be multisig/DAO-held; not enforced on-chain. |

### `relay_upd` — `RelaySignerUpdated`
Emitted by `set_relay_signer()`. Topics: `("relay_upd",)`.

| Field | Type | Notes |
|---|---|---|
| `old_relay_signer` | `Address` | |
| `new_relay_signer` | `Address` | |

### `upgrade_prop` — `UpgradeProposed`
Emitted by `propose_upgrade()`, step 1 of the timelocked upgrade (see
`docs/adr/0003-upgrade-timelock.md`). Topics: `("upgrade_prop",)`.

| Field | Type | Notes |
|---|---|---|
| `new_wasm_hash` | `BytesN<32>` | |
| `expected_schema_version` | `u32` | Checked again at execution time. |
| `earliest_ledger` | `u32` | `execute_upgrade()` fails before this ledger sequence number. |

### `upgrade` — `ContractUpgraded`
Emitted by `execute_upgrade()`, step 2 of the timelocked upgrade. Topics: `("upgrade",)`.

| Field | Type | Notes |
|---|---|---|
| `new_wasm_hash` | `BytesN<32>` | |
| `old_schema_version` | `u32` | |
| `new_schema_version` | `u32` | Always `old_schema_version + 1`. |

## Transaction lifecycle emission order

For a single `transaction_id`, events are emitted in exactly one of these
orders (see `storage::assert_transition` for the state machine that
enforces this):

```
tx_reg -> tx_conf -> tx_comp                 (happy path)
tx_reg -> tx_fail                            (failed while Pending)
tx_reg -> tx_conf -> tx_fail                 (failed while Confirmed)
tx_reg -> tx_refund                          (refunded while Pending)
tx_reg -> tx_conf -> tx_refund               (refunded while Confirmed)
```

`tx_comp`, `tx_fail`, and `tx_refund` are terminal: no further transaction
lifecycle event will ever follow for that `transaction_id`.

## §5. Versioning policy

`schema_version` (returned by `PulsarCoreContract::schema_version()`,
bumped by `execute_upgrade()`) tracks this document, not just the
`Transaction` struct's on-chain layout.

- **Additive / backward-compatible** (new event, new optional-to-ignore
  field appended to an existing event's payload): minor/patch bump. Ship the
  code and this doc update in the same change; no advance notice required,
  but tell subscriber teams it happened.
- **Breaking** (removing a topic or field, renaming either, changing a
  field's type, reordering topics, or changing which events can follow which
  in the lifecycle order above): **major** bump. Requires advance notice to
  whoever owns `pulsar-core` and `pulsar-web` (and `pulsar-swap` once it
  exists) before the change ships — open those repos and check what actually
  breaks, per `CLAUDE.md`'s cross-repo coordination section. Since the
  payload format is a self-describing map, adding a field is additive by
  construction; removing or retyping one is not.
