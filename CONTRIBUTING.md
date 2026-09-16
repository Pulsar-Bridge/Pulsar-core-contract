# Contributing

## Setup

This repo pins its toolchain in `rust-toolchain.toml` (`stable`, plus the
`wasm32v1-none` target — the Soroban host's supported WASM target, not
`wasm32-unknown-unknown`). `rustup` will pick it up automatically in this
directory.

## The bar: `make check`

```sh
make check
```

runs, in order: `cargo fmt --check` -> build the release WASM
(`wasm32v1-none`) -> `cargo clippy --all-targets -- -D warnings` -> `cargo
test`. This must be green before any change is "done" — it's the exact bar
CI enforces.

The WASM build has to happen before `clippy`/`test`, not after: the test
suite's `upgrade()` coverage
(`test_upgrade_bumps_schema_version_and_rejects_replay` in `src/test.rs`)
embeds the contract's own compiled WASM via `include_bytes!` to exercise a
real, host-accepted code swap (the host rejects arbitrary bytes — it
requires a valid contract metadata section, which only a real build
produces). If you change contract code and only run `cargo test` directly
without `make wasm`/`make check` first, that embedded fixture will be
stale relative to your changes; it's rebuilt automatically as part of `make
check` via the `wasm` target's dependency on `src/**/*.rs`.

Individual steps, if you want to run them separately:

```sh
make fmt        # cargo fmt (writes)
make fmt-check  # cargo fmt --check (no write)
make wasm       # release build for wasm32v1-none
make clippy     # cargo clippy --all-targets -- -D warnings
make test       # cargo test (depends on `wasm`)
```

## Every new entry point needs

Per `CLAUDE.md`'s working style:

1. An explicit access-control decision — admin (`admin::require_admin`),
   relay signer (`storage::get_relay_signer(&env)?; relay_signer.require_auth();`),
   or public read-only — stated as the **first thing the function does**.
2. Input validation via `validation.rs`'s established patterns (string
   length caps via `validate_string_len`; SEP-23 strkey + CRC16 checks via
   `validate_strkey_ed25519_public_key` for any field that actually is a
   Stellar address supplied as free text — see the note at that function's
   definition for why it currently has no caller).
3. Tests covering both the happy path and the auth-failure path, matching
   the existing test style in `src/test.rs` (a `Harness` with
   `mock_all_auths()`, `try_*` client methods asserted against the specific
   `Error` variant).

## Events

If your change adds, removes, or reshapes an emitted event, read
`EVENTS.md` — specifically §5 — before writing the code. Additive changes
(a new event, a new field) ship in the same change as the `EVENTS.md`
update. Anything else (removal, rename, reorder, type change, or a change
to the lifecycle emission order) needs advance notice to the sibling repos
per `CLAUDE.md`'s cross-repo coordination section before it ships, not
after.

## Docs that move with the code, not after it

- `CHANGELOG.md`'s "Event schema" section, whenever `EVENTS.md`'s version
  changes.
- `THREAT_MODEL.md`, whenever a change closes, reopens, or introduces a
  finding — don't let it silently drift from what the code actually does.
- `docs/adr/`, for any non-obvious decision, following the
  `0001-relay-signer-trust-model.md` format (Status / Context / Decision /
  Consequences).
