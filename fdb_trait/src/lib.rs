pub mod error;

use std::{fmt, sync::Arc};

use async_trait::async_trait;
pub use error::KvError;
use foundationdb::{Database, FdbBindingError};
use serde::{Serialize, de::DeserializeOwned};

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

    /// Find records by secondary uniq index in a given range, if `stop` is `None`, the range goes to the end
    async fn find_by_unique_index_range<T>(
        db: Arc<Database>,
        index_name: &str,
        start: Option<&T>,
        stop: Option<&T>,
        max_results: Option<usize>,
    ) -> Result<(Vec<Self>, Option<T>), KvError>
    where
        T: Serialize + DeserializeOwned + Sync + Sized + Send + Clone;
    /// Find records by secondary uniq index in a given range, if `stop` is `None`, the range goes to the end
    async fn find_by_unique_index_in_trx_range<T>(
        trx: &foundationdb::RetryableTransaction,
        index_name: &str,
        start: Option<&T>,
        stop: Option<&T>,
        max_results: Option<usize>,
    ) -> Result<(Vec<Self>, Option<T>), foundationdb::FdbBindingError>
    where
        T: Serialize + DeserializeOwned + Sync + Sized + Send + Clone;
}

#[cfg(test)]
mod tests {

    use serde::Deserialize;

    use super::*;

    #[test]
    fn test_name() {
        #[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
        struct Ak {
            id: String,
            sk: String,
            state: String,
            tags: Option<Vec<String>>,
            marker: String,
            trusted: bool,
            owner: String,
        }

        #[async_trait::async_trait]
        impl FdbStore for Ak {
            /// Load struct from FDB via primary key identified by `fdb_key` attribute
            async fn load<T>(db: Arc<Database>, key: &T) -> Result<Self, KvError>
            where
                T: Serialize + Sync + Sized,
            {
                todo!()
            }

            /// Load struct from FDB via primary key identified by `fdb_key` attribute in an existing transaction context
            async fn load_in_trx<T>(
                trx: &foundationdb::RetryableTransaction,
                key: &T,
            ) -> Result<Self, foundationdb::FdbBindingError>
            where
                T: Serialize + Sync + Sized,
            {
                todo!()
            }

            /// Save struct and generate all secondary indexes identified by either `fdb_index` or `fdb_unique_index`
            async fn save(&self, db: Arc<Database>) -> Result<(), KvError> {
                todo!()
            }
            /// Save struct and generate all secondary indexes identified by either `fdb_index` or `fdb_unique_index` in an existing transaction context
            async fn save_in_trx(
                &self,
                trx: &foundationdb::RetryableTransaction,
            ) -> Result<(), FdbBindingError> {
                todo!()
            }

            /// Delete struct and clean up secondary indexes
            async fn delete(&self, db: Arc<Database>) -> Result<(), KvError> {
                todo!()
            }
            /// Delete struct and clean up secondary indexes in an existing transaction context
            async fn delete_in_trx(
                &self,
                trx: &foundationdb::RetryableTransaction,
            ) -> Result<(), FdbBindingError> {
                todo!()
            }

            /// Update struct and all secondary indexes
            async fn update(&self, db: Arc<Database>, new_value: Self) -> Result<(), KvError> {
                todo!()
            }
            /// Update struct and all secondary indexes in an existing transaction context
            async fn update_in_trx(
                &self,
                trx: &foundationdb::RetryableTransaction,
                new_value: Self,
            ) -> Result<(), FdbBindingError> {
                todo!()
            }

            /// Find records by secondary index, the index are stored in the form key: fdb_index -> value: Vec<fdb_key>
            async fn find_by_index<T>(
                db: Arc<Database>,
                index_name: &str,
                index_value: T,
            ) -> Result<Vec<Self>, KvError>
            where
                T: Serialize + Sync + Sized + Send + Clone,
            {
                todo!()
            }
            /// Find records by secondary index, the index are stored in the form key: fdb_index -> value: Vec<fdb_key> in an existing transaction context
            async fn find_by_index_in_trx<T>(
                trx: &foundationdb::RetryableTransaction,
                index_name: &str,
                index_value: T,
            ) -> Result<Vec<Self>, foundationdb::FdbBindingError>
            where
                T: Serialize + Sync + Sized + Send + Clone,
            {
                todo!()
            }

            /// Find records by secondary uniq index, the index are stored in the form key: fdb_unique_index -> value: fdb_key
            async fn find_by_unique_index<T>(
                db: Arc<Database>,
                index_name: &str,
                index_value: T,
            ) -> Result<Self, KvError>
            where
                T: Serialize + Sync + Sized + Send + Clone,
            {
                todo!()
            }
            /// Find records by secondary uniq index, the index are stored in the form key: fdb_unique_index -> value: fdb_key in an existing transaction context
            async fn find_by_unique_index_in_trx<T>(
                trx: &foundationdb::RetryableTransaction,
                index_name: &str,
                index_value: T,
            ) -> Result<Self, foundationdb::FdbBindingError>
            where
                T: Serialize + Sync + Sized + Send + Clone,
            {
                todo!()
            }

            /// Find records by secondary uniq index in a given range, if `stop` is `None`, the range goes to the end
            async fn find_by_unique_index_range<T>(
                db: Arc<Database>,
                index_name: &str,
                start: Option<&T>,
                stop: Option<&T>,
                max_results: Option<usize>,
            ) -> Result<(Vec<Self>, Option<T>), KvError>
            where
                T: Serialize + DeserializeOwned + Sync + Sized + Send + Clone,
            {
                todo!()
            }

            async fn find_by_unique_index_in_trx_range<T>(
                trx: &foundationdb::RetryableTransaction,
                index_name: &str,
                start: Option<&T>,
                stop: Option<&T>,
                max_results: Option<usize>,
            ) -> Result<(Vec<Self>, Option<T>), foundationdb::FdbBindingError>
            where
                T: Serialize + DeserializeOwned + Sync + Sized + Send + Clone,
            {
                async move {
                    let index_name =
                        format!("store:{}:unique_index:{}:", stringify!(#name), index_name);
                    let mut start_index_key_bytes = index_name.clone().into_bytes();
                    let mut stop_index_key_bytes = index_name.clone().into_bytes();
                    let index_bytes_first_part_len = start_index_key_bytes.len();
                    let start_key = match start {
                        Some(start) => {
                            let start_index_value: Vec<u8> =
                                rmp_serde::to_vec(&start).map_err(|e| {
                                    foundationdb::FdbBindingError::CustomError(Box::new(
                                        KvError::EncodeError(e),
                                    ))
                                })?;
                            start_index_key_bytes.extend(start_index_value);
                            start_index_key_bytes.as_slice()
                        }
                        None => start_index_key_bytes.as_slice(),
                    };
                    let end_key = match stop {
                        Some(stop) => {
                            let stop_index_value: Vec<u8> =
                                rmp_serde::to_vec(&stop).map_err(|e| {
                                    foundationdb::FdbBindingError::CustomError(Box::new(
                                        KvError::EncodeError(e),
                                    ))
                                })?;
                            stop_index_key_bytes.extend(stop_index_value);

                            stop_index_key_bytes.as_slice()
                        }
                        None => &[index_name.as_bytes(), &[0xFF]].concat(),
                    };
                    // let end_key = [index_name.as_bytes(), &[0xFF]].concat();
                    let range_option: foundationdb::RangeOption = foundationdb::RangeOption {
                        limit: max_results.map(|v| v + 1),
                        ..foundationdb::RangeOption::from((start_key, end_key))
                    };
                    let iter = trx.get_range(&range_option, 1, false).await?.into_iter();

                    // Retrieve `Self` values from secondary indexes
                    let mut results: Vec<Self> = Vec::new();
                    let mut last_marker: Option<T> = None;
                    for ele in iter {
                        let value = trx
                            .get(ele.value(), false)
                            .await?
                            .ok_or_else(|| KvError::Empty)?;
                        match rmp_serde::from_slice(&value) {
                            Ok(r) => {
                                results.push(r);
                                let last_marker_bytes =
                                    ele.key().split_at(index_bytes_first_part_len).1;
                                last_marker =
                                    rmp_serde::from_slice(last_marker_bytes).map_err(|e| {
                                        foundationdb::FdbBindingError::CustomError(Box::new(
                                            KvError::DecodeError(e),
                                        ))
                                    })?;
                            }
                            Err(e) => {
                                return Err(foundationdb::FdbBindingError::CustomError(Box::new(
                                    KvError::DecodeError(e),
                                )));
                            }
                        };
                    }

                    // Check if there is remaining keys
                    let has_more_items = match max_results {
                        Some(max) => results.len() > max,
                        None => false,
                    };

                    // Remove first element to avoid cover
                    if start.is_some() && results.len() > 1 {
                        results.remove(0);
                    };

                    // Remove last item to avoid cover
                    if results.len() > 1 && has_more_items {
                        results.truncate(results.len() - 1);
                    };

                    // let last_marker = results.last().map(|ak| ak.marker);
                    Ok((results, last_marker))
                }
                .await
            }
        }

        let ak1 = Ak {
            id: "test1".to_owned(),
            sk: "test1".to_owned(),
            state: "active".to_owned(),
            tags: None,
            marker: "a".to_owned(),
            trusted: false,
            owner: "tintin".to_owned(),
        };

        let ak2 = Ak {
            marker: "b".to_owned(),
            ..ak1.clone()
        };

        let ak3 = Ak {
            marker: "c".to_owned(),
            ..ak1.clone()
        };
    }
}
