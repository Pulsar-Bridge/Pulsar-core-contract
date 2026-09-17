//! Admin-only logic: pause/unpause, admin and relay-signer rotation, and
//! upgrade. Every function here must call `require_admin` as the first thing
//! it does (CLAUDE.md security checklist item 1) — the admin address itself
//! is expected to be a multisig/DAO-controlled Stellar account per
//! DECISIONS.md, so `require_auth()` on it already enforces that threshold.
//!
//! These are plain functions, not `#[contractimpl]` entry points: the
//! `#[contractimpl]` macro generates a single `Client`/`Args` pair per
//! contract struct, so all entry points live in one `impl` block in lib.rs,
//! which delegates to the functions here.

use soroban_sdk::{Address, BytesN, Env};

use crate::errors::Error;
use crate::events;
use crate::storage;

pub fn require_admin(env: &Env) -> Result<Address, Error> {
    let admin = storage::get_admin(env)?;
    admin.require_auth();
    Ok(admin)
}

pub fn pause(env: &Env) -> Result<(), Error> {
    let admin = require_admin(env)?;
    storage::set_paused(env, true);
    events::pause_state_changed(env, &admin, true);
    Ok(())
}

pub fn unpause(env: &Env) -> Result<(), Error> {
    let admin = require_admin(env)?;
    storage::set_paused(env, false);
    events::pause_state_changed(env, &admin, false);
    Ok(())
}

/// Step 1 of 2 for rotating the admin address. Only the current admin can
/// propose; nothing changes until the proposed address itself calls
/// `accept_admin`. This two-step handshake exists so a typo'd or
/// unreachable `new_admin` can't accidentally brick admin control the way a
/// single-step rotation could — see
/// `docs/adr/0002-two-step-admin-transfer.md`. `new_admin` must itself be a
/// multisig/DAO account per DECISIONS.md — this contract has no way to
/// enforce that on-chain, so it is an operational requirement on whoever
/// calls this.
pub fn propose_admin(env: &Env, new_admin: Address) -> Result<(), Error> {
    let old_admin = require_admin(env)?;
    storage::set_pending_admin(env, &new_admin);
    storage::extend_instance_ttl(env);
    events::admin_transfer_proposed(env, &old_admin, &new_admin);
    Ok(())
}

/// Step 2 of 2: the proposed admin confirms the transfer by calling this
/// itself, proving it controls `new_admin` before the rotation takes
/// effect. Fails with `NoPendingAdmin` if `propose_admin` hasn't been called
/// (or was already consumed by a prior `accept_admin`).
pub fn accept_admin(env: &Env) -> Result<(), Error> {
    let old_admin = storage::get_admin(env)?;
    let new_admin = storage::get_pending_admin(env)?;
    new_admin.require_auth();

    storage::set_admin(env, &new_admin);
    storage::clear_pending_admin(env);
    storage::extend_instance_ttl(env);
    events::admin_updated(env, &old_admin, &new_admin);
    Ok(())
}

pub fn set_relay_signer(env: &Env, new_relay_signer: Address) -> Result<(), Error> {
    require_admin(env)?;
    let old_relay_signer = storage::get_relay_signer(env)?;
    storage::set_relay_signer(env, &new_relay_signer);
    storage::extend_instance_ttl(env);
    events::relay_signer_updated(env, &old_relay_signer, &new_relay_signer);
    Ok(())
}

/// Step 1 of 2 for a WASM upgrade: records `new_wasm_hash` as pending,
/// executable no earlier than `UPGRADE_TIMELOCK_LEDGERS` ledgers from now.
/// `expected_schema_version` is checked against the current stored version
/// here (fail fast) and re-checked in `execute_upgrade` (defense-in-depth,
/// same compare-and-swap spirit as the original single-step guard — see
/// `docs/adr/0003-upgrade-timelock.md`). Calling this again before executing
/// overwrites any still-pending proposal and resets its timelock.
pub fn propose_upgrade(
    env: &Env,
    new_wasm_hash: BytesN<32>,
    expected_schema_version: u32,
) -> Result<(), Error> {
    require_admin(env)?;

    let current_version = storage::get_schema_version(env)?;
    if current_version != expected_schema_version {
        return Err(Error::SchemaVersionMismatch);
    }

    let earliest_ledger = env.ledger().sequence() + storage::UPGRADE_TIMELOCK_LEDGERS;
    storage::set_pending_upgrade(
        env,
        &crate::types::PendingUpgrade {
            new_wasm_hash: new_wasm_hash.clone(),
            expected_schema_version,
            earliest_ledger,
        },
    );
    storage::extend_instance_ttl(env);

    events::upgrade_proposed(
        env,
        &new_wasm_hash,
        expected_schema_version,
        earliest_ledger,
    );
    Ok(())
}

/// Step 2 of 2: executes a previously proposed WASM upgrade once its
/// timelock has elapsed. `expected_schema_version` (recorded at proposal
/// time) must still match the currently stored schema version — this is a
/// compare-and-swap, not a read-then-write: on success the stored version is
/// bumped and the pending proposal cleared in the same call, so a second
/// `execute_upgrade()` call (naive retry or attempted re-entrant call) fails
/// with `NoPendingUpgrade` instead of re-running the upgrade. Re-verify this
/// invariant any time `execute_upgrade()` or this module changes (CLAUDE.md
/// security checklist item 3).
pub fn execute_upgrade(env: &Env) -> Result<(), Error> {
    require_admin(env)?;

    let pending = storage::get_pending_upgrade(env)?;
    if env.ledger().sequence() < pending.earliest_ledger {
        return Err(Error::UpgradeTimelockNotElapsed);
    }

    let current_version = storage::get_schema_version(env)?;
    if current_version != pending.expected_schema_version {
        return Err(Error::SchemaVersionMismatch);
    }

    let new_version = current_version + 1;
    storage::set_schema_version(env, new_version);
    storage::clear_pending_upgrade(env);
    env.deployer()
        .update_current_contract_wasm(pending.new_wasm_hash.clone());

    events::contract_upgraded(env, &pending.new_wasm_hash, current_version, new_version);
    Ok(())
}
