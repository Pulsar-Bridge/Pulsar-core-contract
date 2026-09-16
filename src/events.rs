//! Event emission. This module is the only place `env.events().publish(..)`
//! should be called from — topic names, payload field order, and payload
//! types are a locked cross-repo public API once downstream repos subscribe
//! (see EVENTS.md). Don't change a signature here without bumping
//! EVENTS.md's version per its §5.

use soroban_sdk::{symbol_short, Address, BytesN, Env, String};

pub fn initialized(env: &Env, admin: &Address, relay_signer: &Address, schema_version: u32) {
    env.events().publish(
        (symbol_short!("init"),),
        (admin.clone(), relay_signer.clone(), schema_version),
    );
}

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
    env.events().publish(
        (symbol_short!("tx_reg"), transaction_id.clone()),
        (
            sender.clone(),
            recipient.clone(),
            amount,
            source_chain.clone(),
            dest_chain.clone(),
            created_at,
        ),
    );
}

pub fn transaction_confirmed(env: &Env, transaction_id: &String, updated_at: u64) {
    env.events().publish(
        (symbol_short!("tx_conf"), transaction_id.clone()),
        (updated_at,),
    );
}

pub fn transaction_completed(env: &Env, transaction_id: &String, updated_at: u64) {
    env.events().publish(
        (symbol_short!("tx_comp"), transaction_id.clone()),
        (updated_at,),
    );
}

pub fn transaction_failed(
    env: &Env,
    transaction_id: &String,
    reason: &String,
    updated_at: u64,
) {
    env.events().publish(
        (symbol_short!("tx_fail"), transaction_id.clone()),
        (reason.clone(), updated_at),
    );
}

pub fn transaction_refunded(env: &Env, transaction_id: &String, updated_at: u64) {
    env.events().publish(
        (symbol_short!("tx_refund"), transaction_id.clone()),
        (updated_at,),
    );
}

pub fn pause_state_changed(env: &Env, by: &Address, paused: bool) {
    env.events()
        .publish((symbol_short!("pause"),), (by.clone(), paused));
}

pub fn admin_updated(env: &Env, old_admin: &Address, new_admin: &Address) {
    env.events().publish(
        (symbol_short!("admin_upd"),),
        (old_admin.clone(), new_admin.clone()),
    );
}

pub fn relay_signer_updated(env: &Env, old_relay_signer: &Address, new_relay_signer: &Address) {
    env.events().publish(
        (symbol_short!("relay_upd"),),
        (old_relay_signer.clone(), new_relay_signer.clone()),
    );
}

pub fn contract_upgraded(
    env: &Env,
    new_wasm_hash: &BytesN<32>,
    old_schema_version: u32,
    new_schema_version: u32,
) {
    env.events().publish(
        (symbol_short!("upgrade"),),
        (new_wasm_hash.clone(), old_schema_version, new_schema_version),
    );
}
