# Pulsar-core-contract
Soroban smart contract powering Pulsar Bridge — the on-chain source of truth for deposit transactions, tracking their lifecycle from Pending through Completed with a versioned event schema for downstream consumers.

## Status

All entry points below are implemented, tested, and pass `make check`
(fmt, clippy `-D warnings`, the 20-test suite, and a `wasm32v1-none`
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
| `get_transaction` / `get_admin` / `get_relay_signer` / `is_paused` / `schema_version` | public read-only | done |
| `pause` / `unpause` | admin | done |
| `set_admin` | admin | done |
| `set_relay_signer` | admin | done |
| `upgrade` | admin | done |

10 events (`EVENTS.md`), schema version 1.

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
