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

// #[derive(Debug, Serialize, Deserialize, Clone)]
// struct Ak {
//     // #[fdb_key]
//     id: String,
//     sk: String,
//     state: String,
//     tags: Option<Vec<String>>,
//     // #[fdb_unique_index]
//     marker: Ulid,
//     trusted: bool,
//     // #[fdb_index]
//     owner: String,
// }

// #[cfg(test)]
// mod test {
//     use std::sync::{Arc, OnceLock};

//     use async_trait::async_trait;
//     use foundationdb::{Database, FdbBindingError, api::NetworkAutoStop};
//     use serde::{Deserialize, Serialize};
//     use ulid::Ulid;

//     use crate::{FdbStore, KvError};

//     #[tokio::test]
//     async fn test_implem() {
//         static NET: OnceLock<NetworkAutoStop> = OnceLock::new();
//         NET.get_or_init(|| unsafe { foundationdb::boot() });
//         let db = foundationdb::Database::new(None)
//             .map_err(|e| KvError::Open(e.to_string()))
//             .expect("unable to initialize db");
//         let db = Arc::new(db);

//         #[derive(Debug, Serialize, Deserialize, Clone)]
//         struct Ak {
//             // #[fdb_key]
//             id: String,
//             sk: String,
//             state: String,
//             tags: Option<Vec<String>>,
//             // #[fdb_unique_index]
//             marker: Ulid,
//             trusted: bool,
//             // #[fdb_index]
//             owner: String,
//         }

//         /// Convert to `store:{struct_name}:{primary_key_value_in_MsgPack}`
//         fn as_fdb_primary_key<T>(input_key: &T) -> Result<Vec<u8>, KvError>
//         where
//             T: Serialize + Sync + Sized,
//         {
//             let index_key = format!("store:{}:", "ak");
//             let mut key_bytes = index_key.into_bytes();
//             let key: Vec<u8> = rmp_serde::to_vec(input_key)?;
//             key_bytes.extend(key);
//             Ok(key_bytes)
//         }

//         #[async_trait]
//         impl FdbStore for Ak {
//             async fn load<T>(db: Arc<Database>, key: &T) -> Result<Self, KvError>
//             where
//                 T: Serialize + Sync + Sized,
//             {
//                 let result = db
//                     .run(|trx, _maybe_comitted| async move { Self::load_in_trx(&trx, key).await })
//                     .await?;
//                 Ok(result)
//             }

//             async fn load_in_trx<T>(
//                 trx: &foundationdb::RetryableTransaction,
//                 key: &T,
//             ) -> Result<Self, foundationdb::FdbBindingError>
//             where
//                 T: Serialize + Sync + Sized,
//             {
//                 let key_bytes = as_fdb_primary_key(&key)?;
//                 let key_bytes = key_bytes.clone();
//                 async move {
//                     let key_bytes = key_bytes.clone();
//                     let value = trx
//                         .get(&key_bytes, false)
//                         .await?
//                         .ok_or_else(|| KvError::Empty)?;
//                     let result: Self = match rmp_serde::from_slice(&value) {
//                         Ok(r) => r,
//                         Err(e) => {
//                             return Err(foundationdb::FdbBindingError::CustomError(Box::new(
//                                 KvError::DecodeError(e),
//                             )));
//                         }
//                     };
//                     Ok(result)
//                 }
//                 .await
//             }

//             async fn save(&self, db: Arc<Database>) -> Result<(), KvError> {
//                 let commit = db
//                     .run(|trx, _maybe_comitted| async move { self.save_in_trx(&trx).await })
//                     .await;
//                 commit.map_err(KvError::FdbCommitError)
//             }

//             async fn save_in_trx(
//                 &self,
//                 trx: &foundationdb::RetryableTransaction,
//             ) -> Result<(), FdbBindingError> {
//                 {
//                     let key_bytes = as_fdb_primary_key(&self.id)?;
//                     let value = match rmp_serde::to_vec(self) {
//                         Ok(v) => v,
//                         Err(e) => {
//                             return Err(foundationdb::FdbBindingError::new_custom_error(Box::new(
//                                 KvError::from(e),
//                             )));
//                         }
//                     };

//                     trx.set(&key_bytes, &value);

//                     // Create multiple index entries
//                     let index_key = format!("store:{}:index:{}:", "Ak", "owner");
//                     let mut index_key_bytes = index_key.into_bytes();
//                     let index_value = rmp_serde::to_vec(&self.owner).map_err(|e| {
//                         foundationdb::FdbBindingError::CustomError(Box::new(KvError::EncodeError(
//                             e,
//                         )))
//                     })?;
//                     index_key_bytes.extend(index_value);
//                     let index_key_bytes = index_key_bytes.as_slice();

//                     // let index_key_bytes = as_fdb_secondary_key(&self.owner)?;
//                     let index_value = match trx.get(index_key_bytes, false).await? {
//                         Some(existing) => {
//                             let mut existing_index: Vec<String> = rmp_serde::from_slice(&existing)
//                                 .map_err(|e| {
//                                     FdbBindingError::new_custom_error(Box::new(
//                                         KvError::DecodeError(e),
//                                     ))
//                                 })?;
//                             if !existing_index.contains(&self.id) {
//                                 existing_index.push(self.id.clone());
//                             };

//                             rmp_serde::to_vec(&existing_index).map_err(|e| {
//                                 FdbBindingError::new_custom_error(Box::new(KvError::EncodeError(e)))
//                             })?
//                         }
//                         None => {
//                             let new_index = vec![self.id.clone()];

//                             rmp_serde::to_vec(&new_index).map_err(|e| {
//                                 FdbBindingError::new_custom_error(Box::new(KvError::EncodeError(e)))
//                             })?
//                         }
//                     };
//                     trx.set(index_key_bytes, &index_value);

//                     // Create unique index entries
//                     let index_key =
//                         format!("store:{}:unique_index:{}:{}", "Ak", "marker", self.marker);
//                     let index_key_bytes = index_key.as_bytes();
//                     match trx.get(index_key_bytes, false).await? {
//                         Some(_) => Err(foundationdb::FdbBindingError::new_custom_error(Box::new(
//                             KvError::UniqueIndexAlreadyExist,
//                         ))),
//                         None => Ok(()),
//                     }?;

//                     trx.set(index_key_bytes, &key_bytes);

//                     println!("set ok");
//                     Ok(())
//                 }
//             }

//             async fn delete(&self, db: Arc<Database>) -> Result<(), KvError> {
//                 let commit = db
//                     .run(|trx, _maybe_comitted| async move { self.delete_in_trx(&trx).await })
//                     .await;
//                 commit.map_err(KvError::FdbCommitError)
//             }

//             async fn delete_in_trx(
//                 &self,
//                 trx: &foundationdb::RetryableTransaction,
//             ) -> Result<(), foundationdb::FdbBindingError> {
//                 // Delete unique index
//                 let index_key = format!("store:{}:unique_index:{}:{}", "Ak", "marker", self.marker);
//                 let index_key_bytes = index_key.as_bytes();
//                 trx.clear(index_key_bytes);

//                 // Delete from multiple index
//                 let index_key = format!("store:{}:index:{}:{}", "Ak", "owner", self.owner);
//                 let index_key_bytes = index_key.as_bytes();
//                 match trx.get(index_key_bytes, false).await? {
//                     Some(existing) => {
//                         let mut existing_index: Vec<String> = rmp_serde::from_slice(&existing)
//                             .map_err(|e| {
//                                 foundationdb::FdbBindingError::new_custom_error(Box::new(
//                                     KvError::DecodeError(e),
//                                 ))
//                             })?;
//                         if existing_index.contains(&self.id) {
//                             existing_index.retain(|v| v != &self.id);
//                         };
//                         if existing_index.is_empty() {
//                             trx.clear(index_key_bytes);
//                         } else {
//                             let encoded_index =
//                                 rmp_serde::to_vec(&existing_index).map_err(|e| {
//                                     foundationdb::FdbBindingError::new_custom_error(Box::new(
//                                         KvError::EncodeError(e),
//                                     ))
//                                 })?;
//                             trx.set(index_key_bytes, &encoded_index);
//                         }
//                         Ok(())
//                     }
//                     None => Err(foundationdb::FdbBindingError::new_custom_error(Box::new(
//                         KvError::FdbMissingIndex(index_key),
//                     ))),
//                 }?;
//                 // Delete object itself
//                 let key = format!("store:{}:{}", "ak", self.id);
//                 trx.clear(key.as_bytes());
//                 Ok(())
//             }

//             async fn update(&self, db: Arc<Database>, new_value: Self) -> Result<(), KvError> {
//                 db.run(|trx, _maybe_comitted| {
//                     let value = new_value.clone();
//                     async move {
//                         value.save_in_trx(&trx).await?;
//                         Ok(())
//                     }
//                 })
//                 .await
//                 .map_err(KvError::FdbCommitError)
//             }

//             async fn update_in_trx(
//                 &self,
//                 trx: &foundationdb::RetryableTransaction,
//                 new_value: Self,
//             ) -> Result<(), FdbBindingError> {
//                 todo!()
//             }

//             async fn find_by_index<T>(
//                 db: Arc<Database>,
//                 index_name: &str,
//                 index_value: T,
//             ) -> Result<Vec<Self>, KvError>
//             where
//                 T: Serialize + Sync + Sized + Clone + Send,
//             {
//                 let results = db
//                     .run(|trx, _maybe_comitted| {
//                         let index_value = index_value.clone();
//                         async move {
//                             let index_key = format!("store:{}:index:{}:", "Ak", index_name);
//                             let mut index_key_bytes = index_key.clone().into_bytes();
//                             let index_value = rmp_serde::to_vec(&index_value).map_err(|e| {
//                                 foundationdb::FdbBindingError::CustomError(Box::new(
//                                     KvError::EncodeError(e),
//                                 ))
//                             })?;
//                             index_key_bytes.extend(index_value);
//                             let index_key_bytes = index_key_bytes.as_slice();

//                             // store:Ak:index:owner:\xa6Tintin
//                             let index = match trx.get(index_key_bytes, false).await? {
//                                 Some(index) => {
//                                     let existing_index: Vec<String> = rmp_serde::from_slice(&index)
//                                         .map_err(|e| {
//                                             FdbBindingError::new_custom_error(Box::new(
//                                                 KvError::DecodeError(e),
//                                             ))
//                                         })?;
//                                     Ok(existing_index)
//                                 }
//                                 None => Err(FdbBindingError::new_custom_error(Box::new(
//                                     KvError::FdbMissingIndex(index_key.clone()),
//                                 ))),
//                             }?;

//                             let mut results: Vec<Self> = Vec::new();
//                             for ele in index {
//                                 // convert as primary key
//                                 let primary_key = as_fdb_primary_key(&ele).map_err(|e| {
//                                     foundationdb::FdbBindingError::new_custom_error(Box::new(e))
//                                 })?;
//                                 let value = match trx.get(&primary_key, false).await? {
//                                     Some(byte_value) => {
//                                         let value: Self = rmp_serde::from_slice(&byte_value)
//                                             .map_err(|e| {
//                                                 FdbBindingError::new_custom_error(Box::new(
//                                                     KvError::DecodeError(e),
//                                                 ))
//                                             })?;
//                                         Ok(value)
//                                     }
//                                     None => Err(FdbBindingError::new_custom_error(Box::new(
//                                         KvError::FdbMissingIndex(ele),
//                                     ))),
//                                 }?;
//                                 results.push(value);
//                             }

//                             Ok(results)
//                         }
//                     })
//                     .await?;
//                 Ok(results)
//             }

//             async fn find_by_index_in_trx<T>(
//                 trx: &foundationdb::RetryableTransaction,
//                 index_name: &str,
//                 index_value: T,
//             ) -> Result<Vec<Self>, foundationdb::FdbBindingError>
//             where
//                 T: Serialize + Sync + Sized + Send + Clone,
//             {
//                 todo!()
//             }

//             async fn find_by_unique_index<T>(
//                 db: Arc<Database>,
//                 index_name: &str,
//                 index_value: T,
//             ) -> Result<Self, KvError>
//             where
//                 T: Serialize + Sync + Sized + Clone + Send,
//             {
//                 let value = db
//                     .run(|trx, _maybe_comitted| {
//                         let index_value = index_value.clone();
//                         async move {
//                             let index_key =
//                                 format!("store:{}:unique_index:{}:", stringify!(#name), index_name);
//                             let mut index_key_bytes = index_key.clone().into_bytes();
//                             let index_value = rmp_serde::to_vec(&index_value).map_err(|e| {
//                                 foundationdb::FdbBindingError::CustomError(Box::new(
//                                     KvError::EncodeError(e),
//                                 ))
//                             })?;
//                             index_key_bytes.extend(index_value);
//                             let index_key_bytes = index_key_bytes.as_slice();

//                             let primary_key = trx
//                                 .get(index_key_bytes, false)
//                                 .await
//                                 .map_err(|e| {
//                                     foundationdb::FdbBindingError::new_custom_error(Box::new(
//                                         KvError::from(e),
//                                     ))
//                                 })?
//                                 .ok_or_else(|| KvError::FdbNotFound)?;
//                             let value = match trx.get(&primary_key, false).await? {
//                                 Some(byte_value) => {
//                                     let value: Self =
//                                         rmp_serde::from_slice(&byte_value).map_err(|e| {
//                                             foundationdb::FdbBindingError::new_custom_error(
//                                                 Box::new(KvError::DecodeError(e)),
//                                             )
//                                         })?;
//                                     Ok(value)
//                                 }
//                                 None => Err(foundationdb::FdbBindingError::new_custom_error(
//                                     Box::new(KvError::FdbMissingIndex(index_name.to_string())),
//                                 )),
//                             }?;

//                             Ok(value)
//                         }
//                     })
//                     .await;
//                 value.map_err(KvError::FdbCommitError)
//             }

//             async fn find_by_unique_index_in_trx<T>(
//                 trx: &foundationdb::RetryableTransaction,
//                 index_name: &str,
//                 index_value: T,
//             ) -> Result<Self, foundationdb::FdbBindingError>
//             where
//                 T: Serialize + Sync + Sized + Send + Clone,
//             {
//                 todo!()
//             }
//         }

//         let test_ak1 = Ak {
//             id: "MRVZGWIK8LIKVIFN142H".to_string(),
//             sk: "SHJ9PKH0D6DYSRREMPRIYA69MD0ATZ0KU0FQJXR1".to_string(),
//             state: "ACTIVE".to_string(),
//             tags: None,
//             marker: ulid::Ulid::from_string("01JSECJAGDKM8H302Z2Q358VY3").unwrap(),
//             trusted: true,
//             owner: "Tintin".to_string(),
//         };

//         let r = test_ak1.save(db.clone()).await;
//         println!("{:#?}", r);

//         let test_ak2 = Ak {
//             id: "E9WVQZKKSEXKHMIFXTXH".to_string(),
//             sk: "YKTN5IJVSL8JX3CJYTRJLXFQHH4FISLSE7L1YHSZ".to_string(),
//             state: "ACTIVE".to_string(),
//             tags: None,
//             marker: ulid::Ulid::from_string("01JSPYYQH0Q669HH9CTY44VBDN").unwrap(),
//             trusted: true,
//             owner: "Tintin".to_string(),
//         };

//         let r = test_ak2.save(db.clone()).await;
//         println!("{:#?}", r);

//         let r = test_ak1.delete(db.clone()).await;
//         println!("{:#?}", r);

//         let find_result = Ak::find_by_unique_index(
//             db.clone(),
//             "marker",
//             "01JSPYYQH0Q669HH9CTY44VBDN".to_string(),
//         )
//         .await;
//         println!("Find by unique index: {:#?}", find_result);
//     }
// }
