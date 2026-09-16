//! Event definitions. This module is the only place a `#[contractevent]`
//! struct should be published from — topic names, payload field names/types,
//! and emission order are a locked cross-repo public API once downstream
//! repos subscribe (see EVENTS.md). Don't change a struct here without
//! bumping EVENTS.md's version per its §5.
//!
//! Each event's payload is a self-describing `Symbol -> Val` map (the
//! `#[contractevent]` default), keyed by the field names below, so
//! downstream decoders read fields by name rather than by position.

use soroban_sdk::{contractevent, Address, BytesN, Env, String};

#[contractevent(topics = ["init"])]
pub struct Initialized {
    pub admin: Address,
    pub relay_signer: Address,
    pub schema_version: u32,
}

#[contractevent(topics = ["tx_reg"])]
pub struct TransactionRegistered {
    #[topic]
    pub transaction_id: String,
    pub sender: String,
    pub recipient: Address,
    pub amount: i128,
    pub source_chain: String,
    pub dest_chain: String,
    pub created_at: u64,
}

#[contractevent(topics = ["tx_conf"])]
pub struct TransactionConfirmed {
    #[topic]
    pub transaction_id: String,
    pub updated_at: u64,
}

#[contractevent(topics = ["tx_comp"])]
pub struct TransactionCompleted {
    #[topic]
    pub transaction_id: String,
    pub updated_at: u64,
}

#[contractevent(topics = ["tx_fail"])]
pub struct TransactionFailed {
    #[topic]
    pub transaction_id: String,
    pub reason: String,
    pub updated_at: u64,
}

#[contractevent(topics = ["tx_refund"])]
pub struct TransactionRefunded {
    #[topic]
    pub transaction_id: String,
    pub updated_at: u64,
}

#[contractevent(topics = ["pause"])]
pub struct PauseStateChanged {
    pub by: Address,
    pub paused: bool,
}

#[contractevent(topics = ["admin_prop"])]
pub struct AdminTransferProposed {
    pub old_admin: Address,
    pub proposed_admin: Address,
}

#[contractevent(topics = ["admin_upd"])]
pub struct AdminUpdated {
    pub old_admin: Address,
    pub new_admin: Address,
}

#[contractevent(topics = ["relay_upd"])]
pub struct RelaySignerUpdated {
    pub old_relay_signer: Address,
    pub new_relay_signer: Address,
}

#[contractevent(topics = ["upgrade_prop"])]
pub struct UpgradeProposed {
    pub new_wasm_hash: BytesN<32>,
    pub expected_schema_version: u32,
    pub earliest_ledger: u32,
}

#[contractevent(topics = ["upgrade"])]
pub struct ContractUpgraded {
    pub new_wasm_hash: BytesN<32>,
    pub old_schema_version: u32,
    pub new_schema_version: u32,
}

pub fn initialized(env: &Env, admin: &Address, relay_signer: &Address, schema_version: u32) {
    Initialized {
        admin: admin.clone(),
        relay_signer: relay_signer.clone(),
        schema_version,
    }
    .publish(env);
}

#[allow(clippy::too_many_arguments)]
pub fn transaction_registered(
    env: &Env,
    transaction_id: &String,
    sender: &String,
    recipient: &Address,
    amount: i128,
    source_chain: &String,
    dest_chain: &String,
    created_at: u64,
) {
    TransactionRegistered {
        transaction_id: transaction_id.clone(),
        sender: sender.clone(),
        recipient: recipient.clone(),
        amount,
        source_chain: source_chain.clone(),
        dest_chain: dest_chain.clone(),
        created_at,
    }
    .publish(env);
}

pub fn transaction_confirmed(env: &Env, transaction_id: &String, updated_at: u64) {
    TransactionConfirmed {
        transaction_id: transaction_id.clone(),
        updated_at,
    }
    .publish(env);
}

pub fn transaction_completed(env: &Env, transaction_id: &String, updated_at: u64) {
    TransactionCompleted {
        transaction_id: transaction_id.clone(),
        updated_at,
    }
    .publish(env);
}

pub fn transaction_failed(env: &Env, transaction_id: &String, reason: &String, updated_at: u64) {
    TransactionFailed {
        transaction_id: transaction_id.clone(),
        reason: reason.clone(),
        updated_at,
    }
    .publish(env);
}

pub fn transaction_refunded(env: &Env, transaction_id: &String, updated_at: u64) {
    TransactionRefunded {
        transaction_id: transaction_id.clone(),
        updated_at,
    }
    .publish(env);
}

pub fn pause_state_changed(env: &Env, by: &Address, paused: bool) {
    PauseStateChanged {
        by: by.clone(),
        paused,
    }
    .publish(env);
}

pub fn admin_transfer_proposed(env: &Env, old_admin: &Address, proposed_admin: &Address) {
    AdminTransferProposed {
        old_admin: old_admin.clone(),
        proposed_admin: proposed_admin.clone(),
    }
    .publish(env);
}

pub fn admin_updated(env: &Env, old_admin: &Address, new_admin: &Address) {
    AdminUpdated {
        old_admin: old_admin.clone(),
        new_admin: new_admin.clone(),
    }
    .publish(env);
}

pub fn relay_signer_updated(env: &Env, old_relay_signer: &Address, new_relay_signer: &Address) {
    RelaySignerUpdated {
        old_relay_signer: old_relay_signer.clone(),
        new_relay_signer: new_relay_signer.clone(),
    }
    .publish(env);
}

pub fn upgrade_proposed(
    env: &Env,
    new_wasm_hash: &BytesN<32>,
    expected_schema_version: u32,
    earliest_ledger: u32,
) {
    UpgradeProposed {
        new_wasm_hash: new_wasm_hash.clone(),
        expected_schema_version,
        earliest_ledger,
    }
    .publish(env);
}

pub fn contract_upgraded(
    env: &Env,
    new_wasm_hash: &BytesN<32>,
    old_schema_version: u32,
    new_schema_version: u32,
) {
    ContractUpgraded {
        new_wasm_hash: new_wasm_hash.clone(),
        old_schema_version,
        new_schema_version,
    }
    .publish(env);
}
