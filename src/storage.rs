//! Storage helpers. All reads/writes to instance, persistent, and temporary
//! storage go through here so the TTL policy and the transaction state
//! machine stay in one place.

use soroban_sdk::{Address, Env, String};

use crate::errors::Error;
use crate::types::{StorageKey, Transaction, TransactionStatus};

// Instance storage (admin/config) TTL: extended on every admin write so the
// contract's own config never expires while it's actively administered.
const INSTANCE_TTL_THRESHOLD: u32 = 17_280 * 7; // ~7 days of margin
const INSTANCE_TTL_EXTEND_TO: u32 = 17_280 * 30; // ~30 days

// Persistent transaction records: bridge deposits must survive far longer
// than any single relay run, so they get a generous TTL extended on write.
const TX_TTL_THRESHOLD: u32 = 17_280 * 30; // ~30 days
const TX_TTL_EXTEND_TO: u32 = 17_280 * 365; // ~1 year

// Temporary idempotency fence for register_callback: only needs to survive
// the window in which the relay might retry a delivery, per CLAUDE.md's
// "~24h TTL" note. Ledger close is ~5s, so 24h ≈ 17,280 ledgers.
const CALLBACK_SEEN_TTL: u32 = 17_280;

pub fn extend_instance_ttl(env: &Env) {
    env.storage()
        .instance()
        .extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_EXTEND_TO);
}

pub fn has_admin(env: &Env) -> bool {
    env.storage().instance().has(&StorageKey::Admin)
}

pub fn get_admin(env: &Env) -> Result<Address, Error> {
    env.storage()
        .instance()
        .get(&StorageKey::Admin)
        .ok_or(Error::NotInitialized)
}

pub fn set_admin(env: &Env, admin: &Address) {
    env.storage().instance().set(&StorageKey::Admin, admin);
}

pub fn get_pending_admin(env: &Env) -> Result<Address, Error> {
    env.storage()
        .instance()
        .get(&StorageKey::PendingAdmin)
        .ok_or(Error::NoPendingAdmin)
}

pub fn set_pending_admin(env: &Env, pending_admin: &Address) {
    env.storage()
        .instance()
        .set(&StorageKey::PendingAdmin, pending_admin);
}

pub fn clear_pending_admin(env: &Env) {
    env.storage().instance().remove(&StorageKey::PendingAdmin);
}

pub fn get_relay_signer(env: &Env) -> Result<Address, Error> {
    env.storage()
        .instance()
        .get(&StorageKey::RelaySigner)
        .ok_or(Error::NotInitialized)
}

pub fn set_relay_signer(env: &Env, relay_signer: &Address) {
    env.storage()
        .instance()
        .set(&StorageKey::RelaySigner, relay_signer);
}

pub fn is_paused(env: &Env) -> bool {
    env.storage()
        .instance()
        .get(&StorageKey::Paused)
        .unwrap_or(false)
}

pub fn set_paused(env: &Env, paused: bool) {
    env.storage().instance().set(&StorageKey::Paused, &paused);
}

pub fn require_not_paused(env: &Env) -> Result<(), Error> {
    if is_paused(env) {
        return Err(Error::ContractPaused);
    }
    Ok(())
}

pub fn get_schema_version(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get(&StorageKey::SchemaVersion)
        .unwrap_or(crate::types::SCHEMA_VERSION)
}

pub fn set_schema_version(env: &Env, version: u32) {
    env.storage()
        .instance()
        .set(&StorageKey::SchemaVersion, &version);
}

pub fn has_transaction(env: &Env, transaction_id: &String) -> bool {
    env.storage()
        .persistent()
        .has(&StorageKey::Transaction(transaction_id.clone()))
}

pub fn get_transaction(env: &Env, transaction_id: &String) -> Result<Transaction, Error> {
    env.storage()
        .persistent()
        .get(&StorageKey::Transaction(transaction_id.clone()))
        .ok_or(Error::TransactionNotFound)
}

pub fn set_transaction(env: &Env, tx: &Transaction) {
    let key = StorageKey::Transaction(tx.transaction_id.clone());
    env.storage().persistent().set(&key, tx);
    env.storage()
        .persistent()
        .extend_ttl(&key, TX_TTL_THRESHOLD, TX_TTL_EXTEND_TO);
}

/// The state machine every status transition must obey:
/// Pending -> Confirmed -> Completed
/// Pending | Confirmed -> Failed
/// Pending | Confirmed -> Refunded
/// Completed, Failed, and Refunded are terminal.
pub fn assert_transition(from: &TransactionStatus, to: &TransactionStatus) -> Result<(), Error> {
    use TransactionStatus::*;
    let allowed = matches!(
        (from, to),
        (Pending, Confirmed)
            | (Pending, Failed)
            | (Pending, Refunded)
            | (Confirmed, Completed)
            | (Confirmed, Failed)
            | (Confirmed, Refunded)
    );
    if allowed {
        Ok(())
    } else {
        Err(Error::InvalidStateTransition)
    }
}

/// Idempotency for `register_callback`. Two independent guards, by design
/// (defense-in-depth per CLAUDE.md — preserve both if this is touched):
///
/// 1. A short-TTL temporary-storage fence (`CallbackSeen`) that catches
///    same-window retries cheaply.
/// 2. A durable check that the transaction already has a stored record whose
///    status is already `Completed` — this catches replays *after* the
///    temporary fence has expired, since temporary storage is not a safe
///    single source of truth for "did this already happen."
///
/// Returns `Ok(true)` if this call is a duplicate (caller should short-circuit
/// and return success without re-emitting events), `Ok(false)` if this is the
/// first time this transaction_id's callback has been seen.
pub fn check_and_mark_callback_seen(
    env: &Env,
    transaction_id: &String,
    already_completed: bool,
) -> bool {
    if already_completed {
        return true;
    }

    let key = StorageKey::CallbackSeen(transaction_id.clone());
    let seen = env.storage().temporary().has(&key);
    if seen {
        return true;
    }

    env.storage().temporary().set(&key, &true);
    env.storage()
        .temporary()
        .extend_ttl(&key, CALLBACK_SEEN_TTL, CALLBACK_SEEN_TTL);
    false
}
