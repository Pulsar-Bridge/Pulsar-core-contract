#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, BytesN as _, Ledger as _},
    Address, BytesN, Env, String,
};

use crate::{Error, PulsarCoreContract, PulsarCoreContractClient, TransactionStatus};

struct Harness<'a> {
    env: Env,
    contract_id: Address,
    client: PulsarCoreContractClient<'a>,
    admin: Address,
    relay_signer: Address,
}

fn setup() -> Harness<'static> {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(PulsarCoreContract, ());
    let client = PulsarCoreContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let relay_signer = Address::generate(&env);
    client.initialize(&admin, &relay_signer);

    Harness {
        env,
        contract_id,
        client,
        admin,
        relay_signer,
    }
}

fn tx_id(env: &Env, s: &str) -> String {
    String::from_str(env, s)
}

fn register_default(h: &Harness) -> String {
    let id = tx_id(&h.env, "tx-1");
    let sender = String::from_str(&h.env, "GABC123SENDERADDR");
    let recipient = Address::generate(&h.env);
    let source_chain = String::from_str(&h.env, "ethereum");
    let dest_chain = String::from_str(&h.env, "stellar");
    h.client.register_transaction(
        &id,
        &sender,
        &recipient,
        &1_000_i128,
        &source_chain,
        &dest_chain,
    );
    id
}

// --- initialize ---

#[test]
fn test_initialize_sets_state() {
    let h = setup();
    assert_eq!(h.client.get_admin(), h.admin);
    assert_eq!(h.client.get_relay_signer(), h.relay_signer);
    assert!(!h.client.is_paused());
    assert_eq!(h.client.schema_version(), 1);
}

#[test]
fn test_initialize_twice_fails() {
    let h = setup();
    let res = h.client.try_initialize(&h.admin, &h.relay_signer);
    assert_eq!(res, Err(Ok(Error::AlreadyInitialized)));
}

// --- register_transaction: happy path + validation + auth ---

#[test]
fn test_register_transaction_happy_path() {
    let h = setup();
    let id = register_default(&h);
    let tx = h.client.get_transaction(&id);
    assert_eq!(tx.status, TransactionStatus::Pending);
    assert_eq!(tx.amount, 1_000_i128);
}

#[test]
fn test_register_transaction_emits_tx_reg_event() {
    // EVENTS.md is a cross-repo contract: prove at least one event actually
    // matches what it documents (topics, field names/types, and that the
    // payload really is the `#[contractevent]` self-describing map), not
    // just that `register_transaction` returns Ok.
    //
    // `env.events().all()` only reflects the *last* contract invocation, so
    // this reads no state via the client after the call under test — any
    // follow-up call (even a read-only query) would replace it with that
    // call's (empty) event list.
    use soroban_sdk::{testutils::Events as _, Event as _};

    let h = setup();
    let id = tx_id(&h.env, "tx-event");
    let sender = String::from_str(&h.env, "GABC123SENDERADDR");
    let recipient = Address::generate(&h.env);
    let source_chain = String::from_str(&h.env, "ethereum");
    let dest_chain = String::from_str(&h.env, "stellar");
    let created_at = h.env.ledger().timestamp();

    h.client.register_transaction(
        &id,
        &sender,
        &recipient,
        &1_000_i128,
        &source_chain,
        &dest_chain,
    );

    let expected = crate::events::TransactionRegistered {
        transaction_id: id,
        sender,
        recipient,
        amount: 1_000_i128,
        source_chain,
        dest_chain,
        created_at,
    };

    assert_eq!(
        h.env.events().all(),
        [expected.to_xdr(&h.env, &h.contract_id)]
    );
}

#[test]
fn test_register_transaction_duplicate_id_fails() {
    let h = setup();
    let id = register_default(&h);
    let sender = String::from_str(&h.env, "GABC123SENDERADDR");
    let recipient = Address::generate(&h.env);
    let source_chain = String::from_str(&h.env, "ethereum");
    let dest_chain = String::from_str(&h.env, "stellar");
    let res = h.client.try_register_transaction(
        &id,
        &sender,
        &recipient,
        &1_000_i128,
        &source_chain,
        &dest_chain,
    );
    assert_eq!(res, Err(Ok(Error::TransactionAlreadyExists)));
}

#[test]
fn test_register_transaction_rejects_non_positive_amount() {
    let h = setup();
    let id = tx_id(&h.env, "tx-bad-amount");
    let sender = String::from_str(&h.env, "GABC123SENDERADDR");
    let recipient = Address::generate(&h.env);
    let source_chain = String::from_str(&h.env, "ethereum");
    let dest_chain = String::from_str(&h.env, "stellar");
    let res = h.client.try_register_transaction(
        &id,
        &sender,
        &recipient,
        &0_i128,
        &source_chain,
        &dest_chain,
    );
    assert_eq!(res, Err(Ok(Error::InvalidInput)));
}

#[test]
fn test_register_transaction_rejects_empty_string_field() {
    let h = setup();
    let id = tx_id(&h.env, "tx-empty-sender");
    let sender = String::from_str(&h.env, "");
    let recipient = Address::generate(&h.env);
    let source_chain = String::from_str(&h.env, "ethereum");
    let dest_chain = String::from_str(&h.env, "stellar");
    let res = h.client.try_register_transaction(
        &id,
        &sender,
        &recipient,
        &1_000_i128,
        &source_chain,
        &dest_chain,
    );
    assert_eq!(res, Err(Ok(Error::InvalidInput)));
}

#[test]
fn test_register_transaction_requires_relay_signer_auth() {
    let h = setup();
    h.env.set_auths(&[]);

    let id = tx_id(&h.env, "tx-unauth");
    let sender = String::from_str(&h.env, "GABC123SENDERADDR");
    let recipient = Address::generate(&h.env);
    let source_chain = String::from_str(&h.env, "ethereum");
    let dest_chain = String::from_str(&h.env, "stellar");
    let res = h.client.try_register_transaction(
        &id,
        &sender,
        &recipient,
        &1_000_i128,
        &source_chain,
        &dest_chain,
    );
    assert!(res.is_err());
}

#[test]
fn test_register_transaction_fails_when_paused() {
    let h = setup();
    h.client.pause();

    let id = tx_id(&h.env, "tx-paused");
    let sender = String::from_str(&h.env, "GABC123SENDERADDR");
    let recipient = Address::generate(&h.env);
    let source_chain = String::from_str(&h.env, "ethereum");
    let dest_chain = String::from_str(&h.env, "stellar");
    let res = h.client.try_register_transaction(
        &id,
        &sender,
        &recipient,
        &1_000_i128,
        &source_chain,
        &dest_chain,
    );
    assert_eq!(res, Err(Ok(Error::ContractPaused)));
}

#[test]
fn test_confirm_transaction_requires_relay_signer_auth() {
    let h = setup();
    let id = register_default(&h);
    h.env.set_auths(&[]);
    let res = h.client.try_confirm_transaction(&id);
    assert!(res.is_err());
    assert_eq!(
        h.client.get_transaction(&id).status,
        TransactionStatus::Pending
    );
}

#[test]
fn test_register_callback_requires_relay_signer_auth() {
    let h = setup();
    let id = register_default(&h);
    h.client.confirm_transaction(&id);
    h.env.set_auths(&[]);
    let res = h.client.try_register_callback(&id);
    assert!(res.is_err());
    assert_eq!(
        h.client.get_transaction(&id).status,
        TransactionStatus::Confirmed
    );
}

#[test]
fn test_fail_transaction_requires_relay_signer_auth() {
    let h = setup();
    let id = register_default(&h);
    h.env.set_auths(&[]);
    let res = h
        .client
        .try_fail_transaction(&id, &String::from_str(&h.env, "bad deposit"));
    assert!(res.is_err());
    assert_eq!(
        h.client.get_transaction(&id).status,
        TransactionStatus::Pending
    );
}

#[test]
fn test_refund_transaction_requires_relay_signer_auth() {
    let h = setup();
    let id = register_default(&h);
    h.env.set_auths(&[]);
    let res = h.client.try_refund_transaction(&id);
    assert!(res.is_err());
    assert_eq!(
        h.client.get_transaction(&id).status,
        TransactionStatus::Pending
    );
}

// --- state machine ---

#[test]
fn test_full_lifecycle_pending_to_completed() {
    let h = setup();
    let id = register_default(&h);

    h.client.confirm_transaction(&id);
    assert_eq!(
        h.client.get_transaction(&id).status,
        TransactionStatus::Confirmed
    );

    h.client.register_callback(&id);
    assert_eq!(
        h.client.get_transaction(&id).status,
        TransactionStatus::Completed
    );
}

#[test]
fn test_cannot_complete_before_confirmed() {
    let h = setup();
    let id = register_default(&h);
    let res = h.client.try_register_callback(&id);
    assert_eq!(res, Err(Ok(Error::InvalidStateTransition)));
}

#[test]
fn test_cannot_confirm_a_completed_transaction() {
    let h = setup();
    let id = register_default(&h);
    h.client.confirm_transaction(&id);
    h.client.register_callback(&id);

    let res = h.client.try_confirm_transaction(&id);
    assert_eq!(res, Err(Ok(Error::InvalidStateTransition)));
}

#[test]
fn test_pending_can_be_failed_and_refunded_respectively() {
    let h = setup();

    let id_fail = register_default(&h);
    h.client
        .fail_transaction(&id_fail, &String::from_str(&h.env, "source chain reorg"));
    assert_eq!(
        h.client.get_transaction(&id_fail).status,
        TransactionStatus::Failed
    );

    let id2 = tx_id(&h.env, "tx-2");
    let sender = String::from_str(&h.env, "GABC123SENDERADDR");
    let recipient = Address::generate(&h.env);
    let source_chain = String::from_str(&h.env, "ethereum");
    let dest_chain = String::from_str(&h.env, "stellar");
    h.client.register_transaction(
        &id2,
        &sender,
        &recipient,
        &500_i128,
        &source_chain,
        &dest_chain,
    );
    h.client.refund_transaction(&id2);
    assert_eq!(
        h.client.get_transaction(&id2).status,
        TransactionStatus::Refunded
    );
}

#[test]
fn test_terminal_states_reject_further_transitions() {
    let h = setup();
    let id = register_default(&h);
    h.client
        .fail_transaction(&id, &String::from_str(&h.env, "bad deposit"));

    let res = h.client.try_confirm_transaction(&id);
    assert_eq!(res, Err(Ok(Error::InvalidStateTransition)));
}

// --- idempotency ---

#[test]
fn test_register_callback_is_idempotent() {
    let h = setup();
    let id = register_default(&h);
    h.client.confirm_transaction(&id);

    h.client.register_callback(&id);
    // Second delivery of the same callback must be a no-op, not an error.
    h.client.register_callback(&id);

    assert_eq!(
        h.client.get_transaction(&id).status,
        TransactionStatus::Completed
    );
}

#[test]
fn test_register_callback_idempotent_after_temp_fence_expires() {
    // THREAT_MODEL.md's F5 previously claimed the durable-status-check path
    // (as opposed to the temporary-storage fast path) "cannot easily" be
    // exercised in tests because there's no way to fast-forward past the
    // ~24h CallbackSeen TTL. That's not actually true: temporary storage
    // TTLs are counted in ledger sequence numbers, not wall-clock time, and
    // `env.ledger().with_mut` can advance the sequence directly.
    let h = setup();
    let id = register_default(&h);
    h.client.confirm_transaction(&id);
    h.client.register_callback(&id);
    assert_eq!(
        h.client.get_transaction(&id).status,
        TransactionStatus::Completed
    );

    // CALLBACK_SEEN_TTL is 17_280 ledgers (src/storage.rs); advance past it
    // so the CallbackSeen temporary-storage entry expires.
    h.env.ledger().with_mut(|li| li.sequence_number += 17_281);

    // A duplicate delivery arriving after the fence has expired must still
    // be caught — by the durable `status == Completed` check, not the
    // (now-expired) temporary fence — and must not error or re-transition.
    h.client.register_callback(&id);
    assert_eq!(
        h.client.get_transaction(&id).status,
        TransactionStatus::Completed
    );
}

// --- pause / admin ---

#[test]
fn test_pause_blocks_relay_actions_but_admin_can_unpause() {
    let h = setup();
    let id = register_default(&h);
    h.client.confirm_transaction(&id);

    h.client.pause();
    let res = h.client.try_register_callback(&id);
    assert_eq!(res, Err(Ok(Error::ContractPaused)));

    h.client.unpause();
    h.client.register_callback(&id);
    assert_eq!(
        h.client.get_transaction(&id).status,
        TransactionStatus::Completed
    );
}

#[test]
fn test_pause_requires_admin_auth() {
    let h = setup();
    h.env.set_auths(&[]);
    let res = h.client.try_pause();
    assert!(res.is_err());
}

#[test]
fn test_unpause_requires_admin_auth() {
    let h = setup();
    h.client.pause();
    h.env.set_auths(&[]);
    let res = h.client.try_unpause();
    assert!(res.is_err());
    assert!(h.client.is_paused());
}

#[test]
fn test_propose_and_accept_admin_transfer() {
    let h = setup();
    let new_admin = Address::generate(&h.env);

    h.client.propose_admin(&new_admin);
    assert_eq!(h.client.get_pending_admin(), new_admin);
    // Not yet rotated: proposing alone must not change the active admin.
    assert_eq!(h.client.get_admin(), h.admin);

    h.client.accept_admin();
    assert_eq!(h.client.get_admin(), new_admin);
    // Pending slot is cleared once consumed.
    let res = h.client.try_get_pending_admin();
    assert_eq!(res, Err(Ok(Error::NoPendingAdmin)));
}

#[test]
fn test_propose_admin_requires_admin_auth() {
    let h = setup();
    let new_admin = Address::generate(&h.env);
    h.env.set_auths(&[]);
    let res = h.client.try_propose_admin(&new_admin);
    assert!(res.is_err());
}

#[test]
fn test_accept_admin_requires_proposed_admin_auth() {
    // accept_admin must be authorized by the *proposed* admin, not the
    // current one — otherwise the current admin could complete the
    // handshake unilaterally, defeating the point of a two-step transfer.
    let h = setup();
    let new_admin = Address::generate(&h.env);
    h.client.propose_admin(&new_admin);

    h.env.set_auths(&[]);
    let res = h.client.try_accept_admin();
    assert!(res.is_err());
    assert_eq!(h.client.get_admin(), h.admin);
}

#[test]
fn test_accept_admin_fails_without_pending_admin() {
    let h = setup();
    let res = h.client.try_accept_admin();
    assert_eq!(res, Err(Ok(Error::NoPendingAdmin)));
}

#[test]
fn test_set_relay_signer_rotates_signer() {
    let h = setup();
    let new_signer = Address::generate(&h.env);
    h.client.set_relay_signer(&new_signer);
    assert_eq!(h.client.get_relay_signer(), new_signer);
}

#[test]
fn test_set_relay_signer_requires_admin_auth() {
    let h = setup();
    let new_signer = Address::generate(&h.env);
    h.env.set_auths(&[]);
    let res = h.client.try_set_relay_signer(&new_signer);
    assert!(res.is_err());
    assert_eq!(h.client.get_relay_signer(), h.relay_signer);
}

// --- upgrade / schema version guard ---

// The contract's own compiled Wasm, used to exercise `upgrade()` with a
// real, host-accepted code blob (the host rejects arbitrary bytes — it
// requires a valid contract metadata section). `make check` builds this
// before `cargo test` runs; see the Makefile.
const SELF_WASM: &[u8] =
    include_bytes!("../target/wasm32v1-none/release/pulsar_core_contract.wasm");

#[test]
fn test_upgrade_bumps_schema_version_and_rejects_replay() {
    let h = setup();
    let new_wasm_hash = h.env.deployer().upload_contract_wasm(SELF_WASM);

    assert_eq!(h.client.schema_version(), 1);
    h.client.upgrade(&new_wasm_hash, &1);
    assert_eq!(h.client.schema_version(), 2);

    // Replaying the upgrade call with the same expected_schema_version must
    // fail now that the guard has advanced — this is the exact scenario
    // CLAUDE.md's security checklist flags: re-verify the guard can't be
    // bypassed by a second call using a stale expected version.
    //
    // The first call above already swapped the contract's executable, so a
    // second call through `h.client` would now dispatch into `MINIMAL_WASM`
    // instead of re-checking our guard. Call the guarded function directly
    // in the contract's storage context instead, which is what the guard
    // itself actually needs to prove.
    let res: Result<(), Error> = h.env.as_contract(&h.contract_id, || {
        crate::admin::upgrade(&h.env, new_wasm_hash.clone(), 1)
    });
    assert_eq!(res, Err(Error::SchemaVersionMismatch));
}

#[test]
fn test_upgrade_requires_admin_auth() {
    let h = setup();
    h.env.set_auths(&[]);
    let new_wasm_hash = BytesN::<32>::random(&h.env);
    let res = h.client.try_upgrade(&new_wasm_hash, &1);
    assert!(res.is_err());
}
