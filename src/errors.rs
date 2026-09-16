use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    NotInitialized = 1,
    AlreadyInitialized = 2,
    Unauthorized = 3,
    ContractPaused = 4,
    InvalidInput = 5,
    InvalidStrkey = 6,
    TransactionNotFound = 7,
    TransactionAlreadyExists = 8,
    InvalidStateTransition = 9,
    SchemaVersionMismatch = 10,
    NoPendingAdmin = 11,
    NoPendingUpgrade = 12,
    UpgradeTimelockNotElapsed = 13,
}
