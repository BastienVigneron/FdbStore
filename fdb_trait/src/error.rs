use std::str::Utf8Error;

use foundationdb::{FdbBindingError, FdbError, TransactionCommitError};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum KvError {
    #[error("unable to open database: {0}")]
    Open(String),
    #[error("unable to destroy database: {0}")]
    Destroy(String),
    #[error("unable to commit transaction: {0}")]
    Commit(String),
    #[error("unable to persist key: {0}")]
    Persist(String),
    #[error("unable to get key: {0}")]
    Get(String),
    #[error("unable to update key: {0}")]
    Update(String),
    #[error("unable to found value with this key")]
    Load,
    #[error("the requested key is not found")]
    Empty,
    #[error("unable to lock KV store: {0}")]
    Lock(String),
    #[error("unable to execute command: {0}")]
    GenericError(String),
    #[error("unable to initialize KV store: {0}")]
    #[allow(dead_code)]
    Init(String),
    #[error("unable to delete value from store: {0}")]
    Delete(String),
    #[error("conversion error")]
    ConversionError(#[from] Utf8Error),
    #[error("foundationDB error")]
    FdbError(#[from] FdbError),
    #[error("foundationDB error")]
    FdbTransactionError(#[from] TransactionCommitError),
    #[error("encode error")]
    EncodeError(#[from] rmp_serde::encode::Error),
    #[error("decode error")]
    DecodeError(#[from] rmp_serde::decode::Error),
    #[error("commit error")]
    FdbCommitError(#[from] FdbBindingError),
    #[error("index not found")]
    FdbMissingIndex,
    #[error("primary key value not found")]
    FdbPrimaryKeyValueNotFound,
    #[error("key not found")]
    FdbNotFound,
    #[error("can't update primary key")]
    UpdatePrimaryKey,
    #[error("the new value for update doesn't match primary key")]
    WrongPrimaryKey,
    #[error("unique index already exist")]
    UniqueIndexAlreadyExist,
}

impl From<KvError> for FdbBindingError {
    fn from(value: KvError) -> Self {
        FdbBindingError::CustomError(Box::new(value))
    }
}
