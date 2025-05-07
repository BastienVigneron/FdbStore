# fdb_derive

### Presentation

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
### Motivation

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

### What does it do?

`FdbStore` generates all the code needed to manage:
- Serialization/deserialization (using [Message Pack](https://msgpack.org/)),
- Unique / multiple secondary indexes,
- Create / read / Update / Delete operations.

For any Rust `struct`, only by adding a `Derive` macro and some field annotations.
### Usage

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

### Know limitations
- `FdbStore` doesn't (yet) manage key or value splitting if a key exceeds 10,000 bytes or a value exceeds 100,000 bytes (after serialization).
- `FdbStore` doesn't (yet) manage large transactions (I.E. transaction that exceed 10,000,000 bytes of affected data).
- `FdbStore` doesn't (yet) manage data encryption.
- `FdbStore` doesn't (yet) manage range queries.
- `FdbStore` doesn't (yet) manage multi-tenancy.


License: Apache 2
