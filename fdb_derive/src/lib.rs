/*!
# FdbStore

## Presentation

`FdbStore` is a proc macro that generates boilerplate code that allows you to CRUD Rust structs into/from [FoundationDB](https://www.foundationdb.org/).
FoundationDB is a rock-solid / combat-proven distributed KV Store used by companies like Apple (all iCloud relies on it) or Snowflake.

But FoundationDB **is not** a database, it's just a distributed KV Store.

That means there is no:
- Schema (just Key and Value)
- Database(s) / Namespace(s) (just on global namespace)
- Data serialization / deserialization mechanism (it just stores byte arrays for both keys and values)
- Authentication / user management / security layer (you have to develop it)
- Secondary indexes management (you have to build it on top of keys/values)

FoundationDB just provides:
- A distributed Keys / Values store
- Where each read/write is performed in transaction (with some [limitation](https://apple.github.io/foundationdb/known-limitations.html))
- …that all

But it does very well, at (large) scale, even when network / hardware partially fail.
## Motivation

Considering FoundationDB feature set, using it to persist your program data is a little bit boring, you have to:
- Manage keys and values serialization / deserialization.
- Manage index / secondary indexes, its creations, its updates and its deletions.
- Doing that in the good transaction context to guarantee data integrity.

It quickly becomes annoying and error-prone.

That's why FoundationDB is traditionally used through a [layer](https://apple.github.io/foundationdb/layer-concept.html).

There are some existing [open-source layers](https://github.com/FoundationDB/awesome-foundationdb#layers) that bring you interesting functionalities.

But layers add (at least) one network hop, and there is not yet an interesting layer in my favorite language (Rust) for my needs.

That’s why I developed `FdbStore`.

`FdbStore` is not a full-featured layer but can be used to develop one in Rust or to be directly used in your application.

## What does it do?

`FdbStore` generates all the code needed to manage:
- Serialization/deserialization (using [Message Pack](https://msgpack.org/)),
- Unique / multiple secondary indexes,
- Create / read / Update / Delete operations.

For any Rust `struct`, only by adding a `Derive` macro and some field annotations.
## Usage

Launch it by using :

```rust
#[derive(Debug, Serialize, Deserialize, FdbStore, PartialEq)]
struct Ak {
    #[fdb_key]
    id: String,                 // This will be used as a primary key
    sk: String,
    state: String,
    tags: Option<Vec<String>>,
    #[fdb_unique_index]
    marker: Ulid,               // This will be used as a unique secondary index, one marker -> one primary key
    trusted: bool,
    #[fdb_index]
    owner: String,              // This will be used as a multiple secondary index, one owner -> a Vec< of primary keys >
}
// Let’s create a struct value
let ak1 = Ak {
    id: "4H2EKB28NOXPF6K40QOT".to_string(),
    sk: "EIMEIGHOH2GA5AEM4TAE6JIEROER0INGOOZEACAI".to_string(),
    state: "ACTIVE".to_string(),
    tags: None,
    marker: Ulid::from_str("01JRX2VBGFD15EH6H5H9AD5WC8").unwrap(),
    trusted: true,
    owner: "Bob".to_string(),
};
// Save with indexes
ak1.save(db.clone()).await?;

// Now struct can be retrieved by his primary key
let recorded_ak1 = Ak::load(db.clone(), &"4H2EKB28NOXPF6K40QOT").await?;

// Or by any unique secondary index
let r = Ak::load_by_sk(db.clone(),"EIMEIGHOH2GA5AEM4TAE6JIEROER0INGOOZEACAI".to_string(),).await?;

// Or by any multiple secondary index
let r = Ak::find_by_index(db.clone(), "owner", "Bob".to_string()).await?;

// Update ak1
let new_marker = Ulid::new();
let ak_updated = Ak {
    state: "LOCKED".to_string(),
    marker: new_marker,
    owner: "Alice".to_string(),
    ..ak1.clone()
};
ak1.update(db.clone(), ak_updated.clone()).await?;

// And finally delete it
ak1.delete(db.clone()).await?;
```

Checkout tests in `/tests/src/test.rs` for more examples.

Any kind of type can be used as a primary key / secondary index as long as it implements `Serialize, Deserialize`.

## Know limitations
- `FdbStore` doesn't (yet) manage key or value splitting if a key exceeds 10,000 bytes or a value exceeds 100,000 bytes (after serialization).
- `FdbStore` doesn't (yet) manage large transactions (I.E. transaction that exceed 10,000,000 bytes of affected data).
- `FdbStore` doesn't (yet) manage data encryption.
- `FdbStore` doesn't (yet) manage range queries.
- `FdbStore` doesn't (yet) manage multi-tenancy.

*/

use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, parse_macro_input};

#[proc_macro_derive(FdbStore, attributes(fdb_key, fdb_index, fdb_unique_index))]
pub fn derive_fdb_store(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let lower_name = name.to_string().to_lowercase();

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let as_fdb_primary_key_fn_name =
        syn::Ident::new(&format!("{}_as_fdb_primary_key", lower_name), name.span());

    // Get the fields from the struct
    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => panic!("FdbStore derive only supports structs with named fields"),
        },
        _ => panic!("FdbStore derive only supports structs"),
    };

    // Find the primary key field
    let primary_key_field = fields
        .iter()
        .find(|field| field.attrs.iter().any(|attr| attr.path.is_ident("fdb_key")))
        .expect("No primary key field found. Use #[fdb_key] attribute to specify a primary key");

    let primary_key_ident = &primary_key_field.ident;

    // Find index and unique_index fields
    let index_fields: Vec<_> = fields
        .iter()
        .filter(|field| {
            field
                .attrs
                .iter()
                .any(|attr| attr.path.is_ident("fdb_index"))
        })
        .collect();

    let unique_index_fields: Vec<_> = fields
        .iter()
        .filter(|field| {
            field
                .attrs
                .iter()
                .any(|attr| attr.path.is_ident("fdb_unique_index"))
        })
        .collect();

    // Generate code for creating index keys
    let create_index_keys = index_fields.iter().map(|field| {
        let field_name = &field.ident;
        let index_name = field_name.as_ref().unwrap().to_string();
        let name = name.to_string();
        let primary_field_type = &primary_key_field.ty;

        quote! {
            let index_key = format!("store:{}:index:{}:", #name, #index_name);
            let mut index_key_bytes = index_key.into_bytes();
            let index_value = rmp_serde::to_vec(&self.#field_name).map_err(|e| {
                foundationdb::FdbBindingError::CustomError(Box::new(
                    fdb_trait::KvError::EncodeError(e),
                ))
            })?;
            index_key_bytes.extend(index_value);

            let index_value = match trx.get(&index_key_bytes, false).await? {
                Some(existing) => {
                    let mut existing_index: Vec<#primary_field_type> =
                        rmp_serde::from_slice(&existing).map_err(|e| {
                            foundationdb::FdbBindingError::new_custom_error(Box::new(
                                fdb_trait::KvError::DecodeError(e),
                            ))
                        })?;
                    if !existing_index.contains(&self.#primary_key_ident) {
                        existing_index.push(self.#primary_key_ident.clone());
                    };

                    rmp_serde::to_vec(&existing_index).map_err(|e| {
                        foundationdb::FdbBindingError::new_custom_error(Box::new(
                            fdb_trait::KvError::EncodeError(e),
                        ))
                    })?
                }
                None => {
                    let new_index = vec![self.#primary_key_ident.clone()];

                    rmp_serde::to_vec(&new_index).map_err(|e| {
                        foundationdb::FdbBindingError::new_custom_error(Box::new(
                            fdb_trait::KvError::EncodeError(e),
                        ))
                    })?
                }
            };
            trx.set(&index_key_bytes, &index_value);
        }
    });
    let create_index_keys_for_trx = create_index_keys.clone();

    // Generate code for creating unique index keys
    let create_unique_index_keys = unique_index_fields.iter().map(|field| {
        let field_name = &field.ident;
        let index_name = field_name.as_ref().unwrap().to_string();
        let name = name.to_string();
        quote! {
            let index_key = format!("store:{}:unique_index:{}:", #name, #index_name);
            let mut index_key_bytes = index_key.into_bytes();
            let index_value = rmp_serde::to_vec(&self.#field_name).map_err(|e| {
                foundationdb::FdbBindingError::CustomError(Box::new(
                    fdb_trait::KvError::EncodeError(e),
                ))
            })?;
            index_key_bytes.extend(index_value);
            let index_key_bytes = index_key_bytes.as_slice();
            // check if index already exist
            match trx.get(index_key_bytes, false).await? {
                Some(_) => Err(foundationdb::FdbBindingError::new_custom_error(Box::new(
                    fdb_trait::KvError::UniqueIndexAlreadyExist,
                ))),
                None => Ok(()),
            }?;
            trx.set(index_key_bytes, &key_bytes);
        }
    });
    let create_unique_index_keys_for_trx = create_unique_index_keys.clone();

    // Generate code for searching from index
    let find_by_index_keys_in_trx = {
        let name = name.to_string();
        let primary_field_type = &primary_key_field.ty;

        quote! {
            let index_value = index_value.clone();
            async move {
                let index_key = format!("store:{}:index:{}:", #name, index_name);
                let mut index_key_bytes = index_key.clone().into_bytes();
                let index_value = rmp_serde::to_vec(&index_value).map_err(|e| {
                    foundationdb::FdbBindingError::CustomError(Box::new(
                        fdb_trait::KvError::EncodeError(e),
                    ))
                })?;
                index_key_bytes.extend(index_value);
                let index_key_bytes = index_key_bytes.as_slice();

                // ex: `store:Ak:index:owner:\xa6Tintin`
                let index = match trx.get(index_key_bytes, false).await? {
                    Some(index) => {
                        let existing_index: Vec<#primary_field_type> = rmp_serde::from_slice(&index)
                            .map_err(|e| {
                                foundationdb::FdbBindingError::new_custom_error(Box::new(
                                    fdb_trait::KvError::DecodeError(e),
                                ))
                            })?;
                        Ok(existing_index)
                    }
                    None => Err(foundationdb::FdbBindingError::new_custom_error(Box::new(
                        fdb_trait::KvError::FdbMissingIndex,
                    ))),
                }?;

                let mut results: Vec<Self> = Vec::new();
                for ele in index {
                    // convert as primary key
                    let primary_key = #as_fdb_primary_key_fn_name(&ele).map_err(|e| {
                        foundationdb::FdbBindingError::new_custom_error(Box::new(e))
                    })?;
                    let value = match trx.get(&primary_key, false).await? {
                        Some(byte_value) => {
                            let value: Self = rmp_serde::from_slice(&byte_value)
                                .map_err(|e| {
                                    foundationdb::FdbBindingError::new_custom_error(Box::new(
                                        fdb_trait::KvError::DecodeError(e),
                                    ))
                                })?;
                            Ok(value)
                        }
                        None => Err(foundationdb::FdbBindingError::new_custom_error(Box::new(
                            fdb_trait::KvError::FdbPrimaryKeyValueNotFound,
                        ))),
                    }?;
                    results.push(value);
                }
                Ok(results)
            }
        }
    };

    // Generate code for deleting multiple index keys
    let delete_index_keys = index_fields.iter().map(|field| {
        let field_name = &field.ident;
        let index_name = field_name.as_ref().unwrap().to_string();
        let name = name.to_string();
        let primary_field_type = &primary_key_field.ty;

        quote! {
            let index_key = format!("store:{}:index:{}:", #name, #index_name);
            let mut index_key_bytes = index_key.clone().into_bytes();
            let index_value = rmp_serde::to_vec(&self.#field_name).map_err(|e| {
                foundationdb::FdbBindingError::CustomError(Box::new(
                    fdb_trait::KvError::EncodeError(e),
                ))
            })?;
            index_key_bytes.extend(index_value);
            let index_key_bytes = index_key_bytes.as_slice();

            match trx.get(index_key_bytes, false).await? {
                Some(existing) => {
                    let mut existing_index: Vec<#primary_field_type> =
                        rmp_serde::from_slice(&existing).map_err(|e| {
                            foundationdb::FdbBindingError::new_custom_error(Box::new(
                                fdb_trait::KvError::DecodeError(e),
                            ))
                        })?;
                    if existing_index.contains(&self.#primary_key_ident) {
                        existing_index.retain(|v| v != &self.#primary_key_ident);
                    };
                    if existing_index.is_empty() {
                        trx.clear(index_key_bytes);
                    } else {
                        let encoded_index = rmp_serde::to_vec(&existing_index)
                            .map_err(|e| {
                                foundationdb::FdbBindingError::new_custom_error(Box::new(
                                    fdb_trait::KvError::EncodeError(e),
                                ))
                            })?;
                        trx.set(index_key_bytes, &encoded_index);
                    }
                    Ok(())
                }
                None => {
                    Err(foundationdb::FdbBindingError::new_custom_error(Box::new(fdb_trait::KvError::FdbMissingIndex)))
                }
            }?;
        }
    });
    let delete_index_keys_for_update = delete_index_keys.clone();

    // Generate code for deleting unique index keys
    let delete_unique_index_keys = unique_index_fields.iter().map(|field| {
        let field_name = &field.ident;
        let index_name = field_name.as_ref().unwrap().to_string();
        let name = name.to_string();

        quote! {
            let index_key = format!("store:{}:unique_index:{}:", #name, #index_name);
            let mut index_key_bytes = index_key.into_bytes();
            let index_value = rmp_serde::to_vec(&self.#field_name).map_err(|e| {
                foundationdb::FdbBindingError::CustomError(Box::new(
                    fdb_trait::KvError::EncodeError(e),
                ))
            })?;
            index_key_bytes.extend(index_value);
            let index_key_bytes = index_key_bytes.as_slice();
            trx.clear(index_key_bytes);
        }
    });
    let delete_unique_index_keys_for_update = delete_unique_index_keys.clone();

    // Generate helper methods for loading by unique indexes
    let load_by_unique_index_methods = unique_index_fields.iter().map(|field| {
        let field_name = &field.ident;
        let method_name = format!("load_by_{}", field_name.as_ref().unwrap());
        let method_ident = syn::Ident::new(&method_name, proc_macro2::Span::call_site());
        let field_type = &field.ty;

        quote! {
            pub async fn #method_ident(db: std::sync::Arc<foundationdb::Database>, value: #field_type) -> Result<Self, fdb_trait::KvError> {
                Self::find_by_unique_index(db, stringify!(#field_name), value).await
            }
        }
    });

    // Generate helper methods for getting by ranges
    let getting_by_range_methods = unique_index_fields.iter().map(|field| {
        let field_name = &field.ident;
        let method_name = format!("find_by_unique_index_range_{}", field_name.as_ref().unwrap());
        let method_ident = syn::Ident::new(&method_name, proc_macro2::Span::call_site());
        let field_type = &field.ty;

        quote! {
            pub async fn #method_ident(db: std::sync::Arc<foundationdb::Database>, query: fdb_trait::RangeQuery<#field_type>, ignore_first_result: bool) -> Result<(Vec<Self>, Option<#field_type>), fdb_trait::KvError> {
                Self::find_by_unique_index_range::<#field_type>(db, stringify!(#field_name), query, ignore_first_result).await
            }
        }
    });

    // Get keys as bytes
    let get_unique_index_key_as_bytes = unique_index_fields.iter().map(|field| {
        let field_name = &field.ident;
        let index_name = field_name.as_ref().unwrap().to_string();
        let name = name.to_string();
        let method_name = format!("get_bytes_key_{}", field_name.as_ref().unwrap());
        let method_ident_self = syn::Ident::new(&method_name, proc_macro2::Span::call_site());
        let method_name_anon = format!("as_bytes_key_{}", field_name.as_ref().unwrap());
        let method_ident_anon = syn::Ident::new(&method_name_anon, proc_macro2::Span::call_site());
        let field_type = &field.ty;

        quote! {
            pub fn #method_ident_self(&self) -> Result<Vec<u8>, fdb_trait::KvError> {
                let index_key = format!("store:{}:unique_index:{}:", #name, #index_name);
                let mut index_key_bytes = index_key.into_bytes();
                let index_value = rmp_serde::to_vec(&self.#field_name).map_err(|e| {
                   fdb_trait::KvError::EncodeError(e)
                })?;
                index_key_bytes.extend(index_value);
                Ok(index_key_bytes)
            }

            pub fn #method_ident_anon(value: #field_type) -> Result<Vec<u8>, fdb_trait::KvError> {
                let index_key = format!("store:{}:unique_index:{}:", #name, #index_name);
                let mut index_key_bytes = index_key.into_bytes();
                let index_value = rmp_serde::to_vec(&value).map_err(|e| {
                   fdb_trait::KvError::EncodeError(e)
                })?;
                index_key_bytes.extend(index_value);
                Ok(index_key_bytes)
            }
        }
    });

    // Implement FdbStore trait
    let expanded = quote! {

        /// Convert to `store:{struct_name}:{primary_key_value_in_MsgPack}`
        pub fn #as_fdb_primary_key_fn_name<T>(input_key: &T) -> Result<Vec<u8>, fdb_trait::KvError>
        where
            T: Serialize + Sync + Sized,
        {
            let index_key = format!("store:{}:", stringify!(#name));
            let mut key_bytes = index_key.into_bytes();
            let key: Vec<u8> = rmp_serde::to_vec(input_key)?;
            key_bytes.extend(key);
            Ok(key_bytes)
        }

        #[automatically_derived]
        #[async_trait::async_trait]
        impl #impl_generics ::fdb_trait::FdbStore for #name #ty_generics #where_clause {
            async fn load<T>(db: std::sync::Arc<foundationdb::Database>, key: &T) -> Result<Self, fdb_trait::KvError>
            where
                T: Serialize + Sync + Sized,
            {
                let commit = db
                    .run(|trx, _maybe_comitted| async move {
                        Self::load_in_trx(&trx, key).await
                    })
                    .await?;
                Ok(commit)
            }

            async fn load_in_trx<T>(
                trx: &foundationdb::RetryableTransaction,
                key: &T,
            ) -> Result<Self, foundationdb::FdbBindingError>
            where
                T: Serialize + Sync + Sized,
            {
                let key_bytes = #as_fdb_primary_key_fn_name(&key)?;
                let key_bytes = key_bytes.clone();
                async move {
                    let key_bytes = key_bytes.clone();
                    let value = trx
                        .get(&key_bytes, false)
                        .await?
                        .ok_or_else(|| fdb_trait::KvError::Empty)?;
                    let result: Self = match rmp_serde::from_slice(&value) {
                        Ok(r) => r,
                        Err(e) => {
                            return Err(foundationdb::FdbBindingError::CustomError(Box::new(
                                fdb_trait::KvError::DecodeError(e),
                            )));
                        }
                    };
                    Ok(result)
                }
                .await
            }

            /// Load struct from FDB via primary key range identified by `fdb_key` attribute
            async fn load_by_range<T>(
                db: Arc<Database>,
                query: RangeQuery<T>,
            ) -> Result<Vec<Self>, KvError>
            where
                T: Serialize + Sync + Sized + std::marker::Send,
            {
                todo!()
            }

            /// Load struct from FDB via primary key range identified by `fdb_key` attribute in an existing transaction context
            async fn load_by_range_in_trx<T>(
                trx: &foundationdb::RetryableTransaction,
                query: RangeQuery<T>,
            ) -> Result<Vec<Self>, foundationdb::FdbBindingError>
            where
                T: Serialize + Sync + Sized + std::marker::Send,
            {
                todo!()
            }


            async fn save(&self, db: std::sync::Arc<foundationdb::Database>) -> Result<(), fdb_trait::KvError> {
                let commit = db.run(|trx, _maybe_comitted| async move {
                    self.save_in_trx(&trx).await
                }).await;
                commit.map_err(fdb_trait::KvError::FdbCommitError)
            }

             async fn save_in_trx(
                &self,
                trx: &foundationdb::RetryableTransaction,
            ) -> Result<(), foundationdb::FdbBindingError> {
                let key_bytes = #as_fdb_primary_key_fn_name(&self.#primary_key_ident)?;
                let value = match rmp_serde::to_vec(self) {
                    Ok(v) => v,
                    Err(e) => {
                        return Err(foundationdb::FdbBindingError::new_custom_error(
                            Box::new(fdb_trait::KvError::from(e)),
                        ));
                    }
                };
                trx.set(&key_bytes, &value);
                #(#create_index_keys_for_trx)*
                #(#create_unique_index_keys_for_trx)*
                Ok(())
            }

            async fn delete(&self, db: std::sync::Arc<foundationdb::Database>) -> Result<(), fdb_trait::KvError> {
                let commit = db.run(|trx, _maybe_comitted| async move {
                    self.delete_in_trx(&trx).await
                }).await;
                commit.map_err(fdb_trait::KvError::FdbCommitError)
            }

            async fn delete_in_trx(
                &self,
                trx: &foundationdb::RetryableTransaction,
            ) -> Result<(), foundationdb::FdbBindingError> {
                    let key_bytes = #as_fdb_primary_key_fn_name(&self.#primary_key_ident)?;
                    let key_bytes = key_bytes.as_slice();
                    #(#delete_index_keys)*
                    #(#delete_unique_index_keys)*
                    trx.clear(key_bytes);
                    Ok(())
            }

            async fn update(&self, db: std::sync::Arc<foundationdb::Database>, new_value: Self) -> Result<(), fdb_trait::KvError> {
                let commit = db.run(|trx, _maybe_comitted| {
                    let new_value = new_value.clone();
                    async move {
                        self.update_in_trx(&trx,new_value).await
                }}).await;
                commit.map_err(fdb_trait::KvError::FdbCommitError)
            }

            async fn update_in_trx(
                &self,
                trx: &foundationdb::RetryableTransaction,
                new_value: Self,
            ) -> Result<(), foundationdb::FdbBindingError> {
                let key_bytes = #as_fdb_primary_key_fn_name(&self.#primary_key_ident)?;
                let key_bytes = key_bytes.as_slice();
                if (&self.#primary_key_ident != &new_value.#primary_key_ident) {
                    return Err(foundationdb::FdbBindingError::CustomError(Box::new(
                        fdb_trait::KvError::WrongPrimaryKey,
                    )));
                }
                let current_value = trx.get(&key_bytes , false).await?.ok_or_else(|| fdb_trait::KvError::FdbNotFound)?;
                #(#delete_index_keys_for_update)*
                #(#delete_unique_index_keys_for_update)*
                trx.clear(key_bytes);
                new_value.save_in_trx(&trx).await?;
                Ok(())
            }


            async fn find_by_index<T>(
                db: std::sync::Arc<foundationdb::Database>,
                index_name: &str,
                index_value: T,
            ) -> Result<Vec<Self>, fdb_trait::KvError>
            where
                T: Serialize + Sync + Sized + Clone + Send,
            {
                 let results = db.run(|trx, _maybe_comitted| {
                    let index_value = index_value.clone();
                    async move {
                        Self::find_by_index_in_trx(&trx, index_name, index_value).await
                    }
                })
                .await?;
                Ok(results)
            }

            async fn find_by_index_in_trx<T>(
                trx: &foundationdb::RetryableTransaction,
                index_name: &str,
                index_value: T,
            ) -> Result<Vec<Self>, foundationdb::FdbBindingError>
            where
                T: Serialize + Sync + Sized + Send + Clone,
            {
                #find_by_index_keys_in_trx.await
            }


            async fn find_by_unique_index<T>(
                db: std::sync::Arc<foundationdb::Database>,
                index_name: &str,
                index_value: T,
            ) -> Result<Self, fdb_trait::KvError>
            where
                T: Serialize + Sync + Sized + Clone + Send,
            {
                let value = db.run(|trx, _maybe_comitted| {
                    let index_value = index_value.clone();
                    async move {
                        Self::find_by_unique_index_in_trx(&trx, index_name, index_value).await
                    }
                })
                .await;
                value.map_err(fdb_trait::KvError::FdbCommitError)
            }

            async fn find_by_unique_index_in_trx<T>(
                trx: &foundationdb::RetryableTransaction,
                index_name: &str,
                index_value: T,
            ) -> Result<Self, foundationdb::FdbBindingError>
            where
                T: Serialize + Sync + Sized + Send + Clone{
                let index_value = index_value.clone();
                async move {
                    let index_key = format!("store:{}:unique_index:{}:", stringify!(#name), index_name);
                    let mut index_key_bytes = index_key.clone().into_bytes();
                    let index_value = rmp_serde::to_vec(&index_value).map_err(|e| {
                        foundationdb::FdbBindingError::CustomError(Box::new(
                            fdb_trait::KvError::EncodeError(e),
                        ))
                    })?;
                    index_key_bytes.extend(index_value);
                    let index_key_bytes = index_key_bytes.as_slice();

                    let primary_key = trx
                        .get(index_key_bytes, false)
                        .await
                        .map_err(|e| {
                            foundationdb::FdbBindingError::new_custom_error(Box::new(
                                fdb_trait::KvError::from(e),
                            ))
                        })?
                        .ok_or_else(|| fdb_trait::KvError::FdbNotFound)?;
                    let value = match trx.get(&primary_key, false).await? {
                        Some(byte_value) => {
                            let value: Self = rmp_serde::from_slice(&byte_value)
                                .map_err(|e| {
                                    foundationdb::FdbBindingError::new_custom_error(Box::new(
                                        fdb_trait::KvError::DecodeError(e),
                                    ))
                                })?;
                            Ok(value)
                        }
                        None => Err(foundationdb::FdbBindingError::new_custom_error(Box::new(
                            fdb_trait::KvError::FdbMissingIndex,
                        ))),
                    }?;
                    Ok(value)
                }.await
            }

            /// Find records by secondary uniq index in a given range.
            async fn find_by_unique_index_range<T>(
                db: Arc<Database>,
                index_name: &str,
                query: fdb_trait::RangeQuery<T>,
                ignore_first_result: bool,
            ) -> Result<(Vec<Self>, Option<T>), fdb_trait::KvError>
            where
                T: Serialize + DeserializeOwned + Sync + Sized + Send + Clone {
                let value = db.run(|trx, _maybe_comitted| {
                    let index_name = index_name.clone();
                    let query = query.clone();
                    async move {
                        Self::find_by_unique_index_in_trx_range(&trx, index_name, query, ignore_first_result).await
                    }
                })
                .await;
                value.map_err(fdb_trait::KvError::FdbCommitError)
            }

            /// Find records by secondary uniq index in a given range
            async fn find_by_unique_index_in_trx_range<T>(
                trx: &foundationdb::RetryableTransaction,
                index_name: &str,
                query: fdb_trait::RangeQuery<T>,
                ignore_first_result: bool,
            ) -> Result<(Vec<Self>, Option<T>), foundationdb::FdbBindingError>
            where
                T: Serialize + DeserializeOwned  + Sync + Sized + Send + Clone{
                async move {
                    let index_name =
                        format!("store:{}:unique_index:{}:", stringify!(#name), index_name);
                    let mut start_index_key_bytes = index_name.clone().into_bytes();
                    let mut stop_index_key_bytes = index_name.clone().into_bytes();
                    let index_bytes_first_part_len = start_index_key_bytes.len();
                    let binding = [index_name.as_bytes(), &[0xFF]].concat();
                    let end_end_key = binding.as_slice();

                    let range_option: foundationdb::RangeOption = match query {
                        RangeQuery::StartAndStop(start, stop) => {
                            let start_index_value: Vec<u8> =
                                rmp_serde::to_vec(&start).map_err(|e| {
                                    foundationdb::FdbBindingError::CustomError(Box::new(
                                        fdb_trait::KvError::EncodeError(e),
                                    ))
                                })?;
                            start_index_key_bytes.extend(start_index_value);

                            let stop_index_value: Vec<u8> =
                                rmp_serde::to_vec(&stop).map_err(|e| {
                                    foundationdb::FdbBindingError::CustomError(Box::new(
                                        fdb_trait::KvError::EncodeError(e),
                                    ))
                                })?;
                            stop_index_key_bytes.extend(stop_index_value);
                            foundationdb::RangeOption {
                                ..foundationdb::RangeOption::from((
                                    start_index_key_bytes.as_slice(),
                                    stop_index_key_bytes.as_slice(),
                                ))
                            }
                        }
                        RangeQuery::StartAndNbResult(start, nb_results) => {
                            let start_index_value: Vec<u8> =
                                rmp_serde::to_vec(&start).map_err(|e| {
                                    foundationdb::FdbBindingError::CustomError(Box::new(
                                        fdb_trait::KvError::EncodeError(e),
                                    ))
                                })?;
                            start_index_key_bytes.extend(start_index_value);
                            let start_key = start_index_key_bytes.as_slice();
                            foundationdb::RangeOption {
                                limit: Some(nb_results),
                                ..foundationdb::RangeOption::from((start_key, end_end_key))
                            }
                        }
                        RangeQuery::NFirstResults(nb_results) => {
                            let start_key = start_index_key_bytes.as_slice();
                            foundationdb::RangeOption {
                                limit: Some(nb_results),
                                ..foundationdb::RangeOption::from((start_key, end_end_key))
                            }
                        }
                        RangeQuery::All => {
                            let start_key = start_index_key_bytes.as_slice();
                            foundationdb::RangeOption {
                                ..foundationdb::RangeOption::from((start_key, end_end_key))
                            }
                        }
                    };

                    let iter = trx.get_range(&range_option, 1, false).await?.into_iter();

                    // Retrieve `Self` values from secondary indexes
                    let mut results: Vec<Self> = Vec::new();
                    let mut last_marker: Option<T> = None;
                    for ele in iter {
                        let value = trx
                            .get(ele.value(), false)
                            .await?
                            .ok_or_else(|| fdb_trait::KvError::Empty)?;
                        match rmp_serde::from_slice(&value) {
                            Ok(r) => {
                                results.push(r);
                                let last_marker_bytes =
                                    ele.key().split_at(index_bytes_first_part_len).1;
                                last_marker =
                                    rmp_serde::from_slice(last_marker_bytes).map_err(|e| {
                                        foundationdb::FdbBindingError::CustomError(Box::new(
                                            fdb_trait::KvError::DecodeError(e),
                                        ))
                                    })?;
                            }
                            Err(e) => {
                                return Err(foundationdb::FdbBindingError::CustomError(Box::new(
                                    fdb_trait::KvError::DecodeError(e),
                                )));
                            }
                        };
                    }

                    // Remove first element to avoid cover
                    if ignore_first_result && results.len() > 1 {
                        results.remove(0);
                    };

                    Ok((results, last_marker))
                }
                .await
            }
        }

        impl #impl_generics #name #ty_generics #where_clause {
            #(#load_by_unique_index_methods)*
            #(#get_unique_index_key_as_bytes )*
            #(#getting_by_range_methods )*
        }
    };

    TokenStream::from(expanded)
}
