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

/// Rotates the admin address. `new_admin` must itself be a multisig/DAO
/// account per DECISIONS.md — this contract has no way to enforce that
/// on-chain, so it is an operational requirement on whoever calls this.
pub fn set_admin(env: &Env, new_admin: Address) -> Result<(), Error> {
    let old_admin = require_admin(env)?;
    storage::set_admin(env, &new_admin);
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

/// Upgrades the contract's WASM. `expected_schema_version` must match the
/// currently stored schema version — this is a compare-and-swap, not a
/// read-then-write: on success the stored version is bumped in the same
/// call, so a second `upgrade()` call with the same `expected_schema_version`
/// (whether a naive retry or an attempted re-entrant call) fails instead of
/// re-running the upgrade. Re-verify this invariant any time `upgrade()` or
/// this module changes (CLAUDE.md security checklist item 3).
pub fn upgrade(
    env: &Env,
    new_wasm_hash: BytesN<32>,
    expected_schema_version: u32,
) -> Result<(), Error> {
    require_admin(env)?;

    let current_version = storage::get_schema_version(env);
    if current_version != expected_schema_version {
        return Err(Error::SchemaVersionMismatch);
    }

    let new_version = current_version + 1;
    storage::set_schema_version(env, new_version);
    env.deployer()
        .update_current_contract_wasm(new_wasm_hash.clone());

    events::contract_upgraded(env, &new_wasm_hash, current_version, new_version);
    Ok(())
}
