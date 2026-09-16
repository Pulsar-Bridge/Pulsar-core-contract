# Pulsar-core-contract
Soroban smart contract powering Pulsar Bridge — the on-chain source of truth for deposit transactions, tracking their lifecycle from Pending through Completed with a versioned event schema for downstream consumers.

## Status

All entry points below are implemented, tested, and pass `make check`
(fmt, clippy `-D warnings`, the 38-test suite, and a `wasm32v1-none`
release build). Not yet independently audited — see `THREAT_MODEL.md` for
open findings.

| Entry point | Access | Status |
|---|---|---|
| `initialize` | none (one-time) | done |
| `register_transaction` | relay signer | done |
| `confirm_transaction` | relay signer | done |
| `register_callback` | relay signer | done |
| `fail_transaction` | relay signer | done |
| `refund_transaction` | relay signer | done |
| `get_transaction` / `get_admin` / `get_pending_admin` / `get_relay_signer` / `is_paused` / `schema_version` / `get_pending_upgrade` | public read-only | done |
| `pause` / `unpause` | admin | done |
| `propose_admin` / `accept_admin` | admin (propose) / proposed admin (accept) | done |
| `set_relay_signer` | admin | done |
| `propose_upgrade` / `execute_upgrade` | admin (timelocked, ~48h between calls) | done |

12 events (`EVENTS.md`), schema version 1.

## Ecosystem

`pulsar-core-contract` is the most mature repo in the Pulsar Bridge system —
the on-chain source of truth that sibling repos consume via `EVENTS.md`'s
versioned event schema.

| Repo | Role | Status |
|---|---|---|
| `pulsar-core-contract` (this repo) | Soroban smart contract — on-chain source of truth for deposit transactions and the versioned event schema downstream repos consume. | Most mature; entry points + events implemented, tested, `make check`-green. |
| `pulsar-core` | Off-chain relay this contract mirrors on-chain. Consumes this contract's events/entry-points. | Clone URL not yet filled in (see `CLAUDE.md.pulsar-core-contracts`). |
| `pulsar-web` | Web frontend/consumer of the bridge. | Clone URL not yet filled in. |
| `pulsar-swap` | Phase 2 consumer; this contract's schema is meant to extend additively for it once its interface is agreed. | Doesn't exist yet. |

To pull the siblings in once their URLs are known:

```sh
git clone <pulsar-core-url> ../pulsar-core
git clone <pulsar-web-url> ../pulsar-web
```

Before changing `EVENTS.md` or any `#[contractimpl]` signature, open the
sibling repos and grep for how they actually consume it — don't assume,
check. Any non-additive change to `EVENTS.md` needs advance notice to
whoever owns `pulsar-core` and `pulsar-web` (and `pulsar-swap` once it
exists) before it ships.

## Docs

- [`EVENTS.md`](./EVENTS.md) — locked event schema for downstream consumers.
- [`DECISIONS.md`](./DECISIONS.md) — standing design decisions.
- [`THREAT_MODEL.md`](./THREAT_MODEL.md) — security findings, open and accepted-risk.
- [`DEPLOYMENT.md`](./DEPLOYMENT.md) — initialize, upgrade, post-deployment checklist.
- [`CONTRIBUTING.md`](./CONTRIBUTING.md) — dev workflow and the `make check` bar.
- [`CHANGELOG.md`](./CHANGELOG.md) — notable changes, including the event-schema history.
- [`docs/adr/`](./docs/adr/) — architecture decision records.
- [`CLAUDE.md.pulsar-core-contracts`](./CLAUDE.md.pulsar-core-contracts) — standing brief for AI-assisted work in this repo.

## Development

```sh
make check   # fmt-check -> wasm build -> clippy -D warnings -> test
```

See `CONTRIBUTING.md` for the full workflow, including why the WASM build
has to run before `clippy`/`test`.
