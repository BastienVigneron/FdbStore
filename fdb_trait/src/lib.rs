pub mod error;

use std::{fmt, sync::Arc};

use async_trait::async_trait;
pub use error::KvError;
use foundationdb::{Database, FdbBindingError};
use serde::Serialize;

/// FdbStore trait define all methods implemented by `fdb_derive` module.
#[async_trait]
pub trait FdbStore: Send + Sync + fmt::Debug + Sized + Clone {
    /// Load struct from FDB via primary key identified by `fdb_key` attribute
    async fn load<T>(db: Arc<Database>, key: &T) -> Result<Self, KvError>
    where
        T: Serialize + Sync + Sized;

    /// Load struct from FDB via primary key identified by `fdb_key` attribute in an existing transaction context
    async fn load_in_trx<T>(
        trx: &foundationdb::RetryableTransaction,
        key: &T,
    ) -> Result<Self, foundationdb::FdbBindingError>
    where
        T: Serialize + Sync + Sized;

    /// Save struct and generate all secondary indexes identified by either `fdb_index` or `fdb_unique_index`
    async fn save(&self, db: Arc<Database>) -> Result<(), KvError>;
    /// Save struct and generate all secondary indexes identified by either `fdb_index` or `fdb_unique_index` in an existing transaction context
    async fn save_in_trx(
        &self,
        trx: &foundationdb::RetryableTransaction,
    ) -> Result<(), FdbBindingError>;

    /// Delete struct and clean up secondary indexes
    async fn delete(&self, db: Arc<Database>) -> Result<(), KvError>;
    /// Delete struct and clean up secondary indexes in an existing transaction context
    async fn delete_in_trx(
        &self,
        trx: &foundationdb::RetryableTransaction,
    ) -> Result<(), FdbBindingError>;

    /// Update struct and all secondary indexes
    async fn update(&self, db: Arc<Database>, new_value: Self) -> Result<(), KvError>;
    /// Update struct and all secondary indexes in an existing transaction context
    async fn update_in_trx(
        &self,
        trx: &foundationdb::RetryableTransaction,
        new_value: Self,
    ) -> Result<(), FdbBindingError>;

    /// Find records by secondary index, the index are stored in the form key: fdb_index -> value: Vec<fdb_key>
    async fn find_by_index<T>(
        db: Arc<Database>,
        index_name: &str,
        index_value: T,
    ) -> Result<Vec<Self>, KvError>
    where
        T: Serialize + Sync + Sized + Send + Clone;
    /// Find records by secondary index, the index are stored in the form key: fdb_index -> value: Vec<fdb_key> in an existing transaction context
    async fn find_by_index_in_trx<T>(
        trx: &foundationdb::RetryableTransaction,
        index_name: &str,
        index_value: T,
    ) -> Result<Vec<Self>, foundationdb::FdbBindingError>
    where
        T: Serialize + Sync + Sized + Send + Clone;

    /// Find records by secondary uniq index, the index are stored in the form key: fdb_unique_index -> value: fdb_key
    async fn find_by_unique_index<T>(
        db: Arc<Database>,
        index_name: &str,
        index_value: T,
    ) -> Result<Self, KvError>
    where
        T: Serialize + Sync + Sized + Send + Clone;
    /// Find records by secondary uniq index, the index are stored in the form key: fdb_unique_index -> value: fdb_key in an existing transaction context
    async fn find_by_unique_index_in_trx<T>(
        trx: &foundationdb::RetryableTransaction,
        index_name: &str,
        index_value: T,
    ) -> Result<Self, foundationdb::FdbBindingError>
    where
        T: Serialize + Sync + Sized + Send + Clone;
}
