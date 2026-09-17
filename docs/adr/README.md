# Architecture Decision Records

Non-obvious design decisions, in the `Status`/`Context`/`Decision`/
`Consequences` format (see `0001` for the template). `DECISIONS.md` at the
repo root is the quick-reference index; these are where the "why" lives.

| ADR | Decision |
|---|---|
| [0001](./0001-relay-signer-trust-model.md) | Why the relay signer is a single hot key (not a multisig) with a deliberately weaker trust model than admin, and what that does and doesn't expose. |
| [0002](./0002-two-step-admin-transfer.md) | Why `set_admin()` was replaced with `propose_admin()`/`accept_admin()`, closing the accidental-lockout risk of a single-step rotation. |
| [0003](./0003-upgrade-timelock.md) | Why `upgrade()` was replaced with a timelocked `propose_upgrade()`/`execute_upgrade()` pair, giving subscribers/signers a window to react before a WASM swap takes effect. |
| [0004](./0004-dependency-vulnerability-scanning.md) | Why dependency vulnerability scanning is a separate, non-blocking CI job (`cargo-audit`) plus `dependabot`, rather than folded into `make check`. |
