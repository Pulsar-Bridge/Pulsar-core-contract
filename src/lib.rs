#![no_std]

mod admin;
mod errors;
mod events;
mod storage;
mod types;
mod validation;

#[cfg(test)]
mod test;

use soroban_sdk::{contract, contractimpl, Address, BytesN, Env, String};

pub use errors::Error;
pub use types::{Transaction, TransactionStatus};

use types::SCHEMA_VERSION;

#[contract]
pub struct PulsarCoreContract;

#[contractimpl]
impl PulsarCoreContract {
    /// One-time setup. `admin` should be a multisig or DAO-controlled Stellar
    /// account per DECISIONS.md — never a single signer, not even on testnet.
    pub fn initialize(env: Env, admin: Address, relay_signer: Address) -> Result<(), Error> {
        if storage::has_admin(&env) {
            return Err(Error::AlreadyInitialized);
        }
        storage::set_admin(&env, &admin);
        storage::set_relay_signer(&env, &relay_signer);
        storage::set_paused(&env, false);
        storage::set_schema_version(&env, SCHEMA_VERSION);
        storage::extend_instance_ttl(&env);

        events::initialized(&env, &admin, &relay_signer, SCHEMA_VERSION);
        Ok(())
    }

    /// Registers a new bridged deposit in `Pending` status. Called by the
    /// off-chain relay once it has observed the source-chain deposit.
    pub fn register_transaction(
        env: Env,
        transaction_id: String,
        sender: String,
        recipient: Address,
        amount: i128,
        source_chain: String,
        dest_chain: String,
    ) -> Result<(), Error> {
        let relay_signer = storage::get_relay_signer(&env)?;
        relay_signer.require_auth();
        storage::require_not_paused(&env)?;

        validation::validate_string_len(&transaction_id)?;
        validation::validate_string_len(&sender)?;
        validation::validate_string_len(&source_chain)?;
        validation::validate_string_len(&dest_chain)?;
        validation::validate_amount(amount)?;

        if storage::has_transaction(&env, &transaction_id) {
            return Err(Error::TransactionAlreadyExists);
        }

        let now = env.ledger().timestamp();
        let tx = Transaction {
            transaction_id: transaction_id.clone(),
            sender: sender.clone(),
            recipient: recipient.clone(),
            amount,
            source_chain: source_chain.clone(),
            dest_chain: dest_chain.clone(),
            status: TransactionStatus::Pending,
            created_at: now,
            updated_at: now,
        };
        storage::set_transaction(&env, &tx);

        events::transaction_registered(
            &env,
            &transaction_id,
            &sender,
            &recipient,
            amount,
            &source_chain,
            &dest_chain,
            now,
        );
        Ok(())
    }

    /// Marks a `Pending` transaction `Confirmed` once the relay has observed
    /// enough source-chain confirmations.
    pub fn confirm_transaction(env: Env, transaction_id: String) -> Result<(), Error> {
        let relay_signer = storage::get_relay_signer(&env)?;
        relay_signer.require_auth();
        storage::require_not_paused(&env)?;

        Self::transition(&env, &transaction_id, TransactionStatus::Confirmed)?;
        let now = env.ledger().timestamp();
        events::transaction_confirmed(&env, &transaction_id, now);
        Ok(())
    }

    /// Idempotent completion callback: marks a `Confirmed` transaction
    /// `Completed` once the destination-chain payout has landed. Guarded by
    /// two independent idempotency checks — see `storage::check_and_mark_callback_seen`.
    /// Preserve both guards if you touch this function.
    pub fn register_callback(env: Env, transaction_id: String) -> Result<(), Error> {
        let relay_signer = storage::get_relay_signer(&env)?;
        relay_signer.require_auth();
        storage::require_not_paused(&env)?;

        let tx = storage::get_transaction(&env, &transaction_id)?;
        let already_completed = tx.status == TransactionStatus::Completed;

        if storage::check_and_mark_callback_seen(&env, &transaction_id, already_completed) {
            // Duplicate delivery: no-op, no re-emitted event.
            return Ok(());
        }

        Self::transition(&env, &transaction_id, TransactionStatus::Completed)?;
        let now = env.ledger().timestamp();
        events::transaction_completed(&env, &transaction_id, now);
        Ok(())
    }

    /// Marks a `Pending` or `Confirmed` transaction `Failed` with a reason.
    pub fn fail_transaction(
        env: Env,
        transaction_id: String,
        reason: String,
    ) -> Result<(), Error> {
        let relay_signer = storage::get_relay_signer(&env)?;
        relay_signer.require_auth();
        storage::require_not_paused(&env)?;
        validation::validate_string_len(&reason)?;

        Self::transition(&env, &transaction_id, TransactionStatus::Failed)?;
        let now = env.ledger().timestamp();
        events::transaction_failed(&env, &transaction_id, &reason, now);
        Ok(())
    }

    /// Marks a `Pending` or `Confirmed` transaction `Refunded`.
    pub fn refund_transaction(env: Env, transaction_id: String) -> Result<(), Error> {
        let relay_signer = storage::get_relay_signer(&env)?;
        relay_signer.require_auth();
        storage::require_not_paused(&env)?;

        Self::transition(&env, &transaction_id, TransactionStatus::Refunded)?;
        let now = env.ledger().timestamp();
        events::transaction_refunded(&env, &transaction_id, now);
        Ok(())
    }

    // --- Read-only queries ---

    pub fn get_transaction(env: Env, transaction_id: String) -> Result<Transaction, Error> {
        storage::get_transaction(&env, &transaction_id)
    }

    pub fn get_admin(env: Env) -> Result<Address, Error> {
        storage::get_admin(&env)
    }

    pub fn get_relay_signer(env: Env) -> Result<Address, Error> {
        storage::get_relay_signer(&env)
    }

    pub fn is_paused(env: Env) -> bool {
        storage::is_paused(&env)
    }

    pub fn schema_version(env: Env) -> u32 {
        storage::get_schema_version(&env)
    }

    // --- Admin-only entry points (logic lives in admin.rs) ---

    pub fn pause(env: Env) -> Result<(), Error> {
        admin::pause(&env)
    }

    pub fn unpause(env: Env) -> Result<(), Error> {
        admin::unpause(&env)
    }

    pub fn set_admin(env: Env, new_admin: Address) -> Result<(), Error> {
        admin::set_admin(&env, new_admin)
    }

    pub fn set_relay_signer(env: Env, new_relay_signer: Address) -> Result<(), Error> {
        admin::set_relay_signer(&env, new_relay_signer)
    }

    pub fn upgrade(
        env: Env,
        new_wasm_hash: BytesN<32>,
        expected_schema_version: u32,
    ) -> Result<(), Error> {
        admin::upgrade(&env, new_wasm_hash, expected_schema_version)
    }

    /// Shared state-machine guard + persist step for every status transition.
    fn transition(
        env: &Env,
        transaction_id: &String,
        to: TransactionStatus,
    ) -> Result<(), Error> {
        let mut tx = storage::get_transaction(env, transaction_id)?;
        storage::assert_transition(&tx.status, &to)?;
        tx.status = to;
        tx.updated_at = env.ledger().timestamp();
        storage::set_transaction(env, &tx);
        Ok(())
    }
}
