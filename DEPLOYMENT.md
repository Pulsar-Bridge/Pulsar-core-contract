# Deployment

Covers initialization, upgrade, and the post-deployment checklist for
`pulsar-core-contract`. Any pipeline/CI work should run exactly these
steps, not reinvent them — per `CLAUDE.md`'s DevOps section.

## Prerequisites

- `stellar` CLI (or `soroban` CLI, depending on your toolchain version)
  configured with a funded source account for the network you're deploying
  to, and network config (`--network testnet|futurenet|mainnet` or
  equivalent) already set up.
- The admin and relay-signer accounts must already exist on-chain before
  `initialize()` is called — this contract doesn't create them.
- `make wasm` (or `make check`) run locally to confirm the build is clean
  before you build the artifact you actually deploy.

## Admin key ceremony (mandatory before any non-testnet deployment)

Per `DECISIONS.md`, the admin address passed to `initialize()` must be a
multisig (≥3-of-5) or DAO-controlled Stellar account — never a single
signer, including on testnet, so that testnet deployments actually exercise
the real signing workflow rather than skipping it.

1. Identify the ≥5 signer holders and set the account's signing threshold
   to require ≥3 of them (a Stellar multisig account: a base account with
   additional `Signer` entries and `low/medium/high` thresholds configured
   accordingly), or use the DAO-controlled account if one is already
   established.
2. Document who holds each signing key and how they're stored (hardware
   wallet, HSM, etc.) — "operationally real," per `CLAUDE.md`, means this is
   a real, current record, not a one-time setup note that goes stale.
3. Verify the multisig account's threshold configuration on-chain (e.g. via
   `stellar account ...` or a block explorer) before using it as the
   `admin` argument to `initialize()` — don't just trust that it was set up
   correctly.
4. Re-run this verification any time `set_admin()` rotates the admin to a
   new account.

## Build

```sh
make wasm
# produces target/wasm32v1-none/release/pulsar_core_contract.wasm
```

## Initialize (first deployment only)

```sh
stellar contract deploy \
  --wasm target/wasm32v1-none/release/pulsar_core_contract.wasm \
  --source <deployer-account> \
  --network <network>
# -> prints the deployed contract id

stellar contract invoke \
  --id <contract-id> \
  --source <deployer-account> \
  --network <network> \
  -- initialize \
  --admin <multisig-or-dao-account> \
  --relay_signer <relay-signer-account>
```

`initialize()` is one-time — a second call fails with
`Error::AlreadyInitialized`. There is no "re-initialize" path; a mistaken
`admin`/`relay_signer` argument must be corrected via `set_admin()` /
`set_relay_signer()` after the fact (which themselves require the
already-set admin's auth), or by deploying a fresh contract instance if the
mistake is in the admin account itself and no valid admin auth is available
to correct it.

## Upgrade

`upgrade()` replaces the currently-deployed contract's executable in place
— the contract's on-chain address, and all its persistent `Transaction`
storage, are preserved. This is **not** available for storage-layout
changes; see `DECISIONS.md`'s "Storage layout changes require a fresh
deployment" entry — if the change you're deploying touches `Transaction` or
`StorageKey`, stop and re-read that entry before proceeding here.

1. Build the new WASM (`make wasm`) from the reviewed, merged code.
2. Upload it without swapping it in yet:
   ```sh
   stellar contract install \
     --wasm target/wasm32v1-none/release/pulsar_core_contract.wasm \
     --source <admin-signer> \
     --network <network>
   # -> prints the new wasm hash
   ```
3. Read the currently stored schema version:
   ```sh
   stellar contract invoke --id <contract-id> --network <network> \
     -- schema_version
   ```
4. Call `upgrade()` with that exact value as `expected_schema_version` —
   this requires all ≥3-of-5 (or DAO) admin signatures per the key ceremony
   above:
   ```sh
   stellar contract invoke \
     --id <contract-id> \
     --source <admin-signer> \
     --network <network> \
     -- upgrade \
     --new_wasm_hash <hash-from-step-2> \
     --expected_schema_version <value-from-step-3>
   ```
   A stale `expected_schema_version` (someone else's upgrade already landed
   since you read it in step 3) fails closed with
   `Error::SchemaVersionMismatch` — re-read the current version and retry
   rather than assuming the call is safe to blindly resubmit.
5. Confirm the new `schema_version()` reads as expected, and confirm the
   `upgrade` event (`EVENTS.md`) was emitted with the expected
   `new_wasm_hash`.

## Post-deployment checklist

- [ ] `initialize()` (or `upgrade()`) transaction confirmed on-chain, not
      just submitted.
- [ ] `get_admin()` and `get_relay_signer()` return the expected accounts.
- [ ] `is_paused()` returns `false` (a fresh deployment should not start
      paused).
- [ ] `schema_version()` matches what was expected for this deployment/upgrade.
- [ ] The `init` (or `upgrade`) event is visible in a block explorer / event
      stream with the expected payload, confirming downstream subscribers
      will actually see it.
- [ ] Contract ID recorded wherever `pulsar-core` and `pulsar-web` read
      their configuration from — an upgrade doesn't change the contract ID,
      but a fresh deployment does, and every sibling repo's config needs to
      be updated in that case.
- [ ] `CHANGELOG.md`'s "Event schema" section updated if this deployment
      shipped an `EVENTS.md` version bump.
