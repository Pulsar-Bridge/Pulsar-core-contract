use soroban_sdk::{contracttype, Address, String};

/// Current on-chain storage/event schema version. Bump per the rules in EVENTS.md
/// whenever a breaking change to `Transaction` or the event payloads ships.
pub const SCHEMA_VERSION: u32 = 1;

/// Maximum length accepted for any free-form chain-identifier or foreign-address
/// string (source/destination chain id, source-chain sender address, memo).
/// Bridged chains are not all Stellar, so these can't be validated as strkeys —
/// a length cap is the guard against unbounded storage growth instead.
pub const MAX_STRING_LEN: u32 = 256;

/// Lifecycle of a bridged deposit, mirrored 1:1 by TransactionRegistered /
/// TransactionConfirmed / TransactionCompleted / TransactionFailed /
/// TransactionRefunded events. This is a state machine: see
/// `storage::assert_transition` for the allowed edges. Adding a variant is a
/// breaking change to the event schema (see EVENTS.md §5).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TransactionStatus {
    Pending,
    Confirmed,
    Completed,
    Failed,
    Refunded,
}

/// The on-chain record of a single bridged deposit. `transaction_id` is the
/// idempotency key: it is the off-chain relay's identifier for the source-chain
/// deposit, not one minted on-chain.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Transaction {
    pub transaction_id: String,
    pub sender: String,
    pub recipient: Address,
    pub amount: i128,
    pub source_chain: String,
    pub dest_chain: String,
    pub status: TransactionStatus,
    pub created_at: u64,
    pub updated_at: u64,
}

/// Storage keys. Any change to variant names/shapes here is a breaking change
/// requiring a fresh deployment, not an in-place upgrade (see CLAUDE.md).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StorageKey {
    /// Multisig/DAO-held admin address (instance storage).
    Admin,
    /// Address proposed by the current admin via `propose_admin()`, not yet
    /// confirmed. Cleared once `accept_admin()` succeeds. Instance storage.
    /// See `docs/adr/0002-two-step-admin-transfer.md`.
    PendingAdmin,
    /// Address authorized to register/confirm/complete/fail/refund transactions
    /// on behalf of the off-chain relay (instance storage).
    RelaySigner,
    /// bool, instance storage.
    Paused,
    /// u32, instance storage. Guards `upgrade()`.
    SchemaVersion,
    /// Durable record of a transaction, keyed by transaction_id. Persistent storage.
    Transaction(String),
    /// Short-TTL idempotency fence for `register_callback`, keyed by transaction_id.
    /// Temporary storage (~24h TTL). See `storage::mark_callback_seen`.
    CallbackSeen(String),
}
