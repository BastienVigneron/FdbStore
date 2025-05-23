/// tests must be launch with:
/// `cargo test -- tests --nocapture --test-threads=1`
/// to avoid any db conflict between them
#[cfg(test)]
mod tests {
    use fdb_derive::FdbStore;
    use fdb_trait::FdbStore;
    use fdb_trait::KvError;
    use foundationdb::api::NetworkAutoStop;
    use foundationdb::{Database, FdbResult};
    use serde::{Deserialize, Serialize};

    use std::str::FromStr;
    use std::sync::Arc;
    use std::sync::OnceLock;
    use ulid::Ulid;

    async fn setup_database() -> FdbResult<Arc<Database>> {
        // Initiate local fdb native client
        static NET: OnceLock<NetworkAutoStop> = OnceLock::new();
        NET.get_or_init(|| unsafe { foundationdb::boot() });
        let db = foundationdb::Database::new(None)
            .map_err(|e| KvError::Open(e.to_string()))
            .expect("unable to initialize db");
        let db = Arc::new(db);
        // Clean DB before tests
        let tx = db.create_trx()?;
        tx.clear_range(b"", b"\xFF");
        tx.commit().await?;
        Ok(db)
    }

    // sample struct to persist
    #[derive(Debug, Serialize, Deserialize, FdbStore, PartialEq, Clone)]
    struct Ak {
        #[fdb_key]
        id: String,
        #[fdb_unique_index]
        sk: String,
        state: String,
        tags: Option<Vec<String>>,
        #[fdb_unique_index]
        marker: Ulid,
        trusted: bool,
        #[fdb_index]
        owner: String,
    }

    #[tokio::test]
    async fn test_01_save() -> Result<(), KvError> {
        let db = setup_database().await?;

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

        // Load from fdb to check equality
        let r = Ak::load(db.clone(), &"4H2EKB28NOXPF6K40QOT").await?;
        assert!(r == ak1);

        // Check we can retrieve by secondary indexes
        let r = Ak::load_by_sk(
            db.clone(),
            "EIMEIGHOH2GA5AEM4TAE6JIEROER0INGOOZEACAI".to_string(),
        )
        .await?;
        assert!(r == ak1);

        // Save a second one
        let ak2 = Ak {
            id: "CKLTKHSLP99NZANMG9RK".to_string(),
            sk: "MNFKU0ZEYSUHUNVSZO8VMQEMLM8XBAYZQGUFMKAG".to_string(),
            state: "ACTIVE".to_string(),
            tags: None,
            marker: Ulid::from_str("01JSPW068HWN9RQH5WCTWXHM2W").unwrap(),
            trusted: true,
            owner: "Bob".to_string(),
        };

        ak2.save(db.clone()).await?;
        let recorded_ak2 = Ak::load(db.clone(), &ak2.id).await?;
        assert_eq!(ak2, recorded_ak2);

        // Check Bob has two aks
        let r = Ak::find_by_index(db.clone(), "owner", "Bob".to_string()).await?;
        assert!(r.len() == 2);

        // Try to record another ak with the same marker must lead to error
        let ak3 = Ak {
            id: "BVFI4ZQ5HH69IOXFNL3S".to_string(),
            sk: "NX5PTZORX3UFEWB7JUPEX3K1FVQYYNVH2MMKQIOE".to_string(),
            state: "ACTIVE".to_string(),
            tags: None,
            marker: Ulid::from_str("01JRX2VBGFD15EH6H5H9AD5WC8").unwrap(),
            trusted: true,
            owner: "Bob".to_string(),
        };
        let r = ak3.save(db.clone()).await;
        assert!(r.is_err());
        Ok(())
    }

    #[tokio::test]
    async fn test_02_delete() -> Result<(), KvError> {
        let db = setup_database().await?;

        let ak1 = Ak {
            id: "4H2EKB28NOXPF6K40QOT".to_string(),
            sk: "EIMEIGHOH2GA5AEM4TAE6JIEROER0INGOOZEACAI".to_string(),
            state: "ACTIVE".to_string(),
            tags: None,
            marker: Ulid::from_str("01JRX2VBGFD15EH6H5H9AD5WC8").unwrap(),
            trusted: true,
            owner: "Bob".to_string(),
        };
        ak1.save(db.clone()).await?;

        // Save a second one
        let ak2 = Ak {
            id: "CKLTKHSLP99NZANMG9RK".to_string(),
            sk: "MNFKU0ZEYSUHUNVSZO8VMQEMLM8XBAYZQGUFMKAG".to_string(),
            state: "ACTIVE".to_string(),
            tags: None,
            marker: Ulid::from_str("01JSPW068HWN9RQH5WCTWXHM2W").unwrap(),
            trusted: true,
            owner: "Bob".to_string(),
        };
        ak2.save(db.clone()).await?;

        // Check Bob has now two ak
        let r = Ak::find_by_index(db.clone(), "owner", "Bob".to_string()).await?;
        assert!(r.len() == 2);

        // Delete ak1
        ak1.delete(db.clone()).await?;

        // Check Bob has now only one ak
        let r = Ak::find_by_index(db.clone(), "owner", "Bob".to_string()).await?;
        assert!(r.len() == 1);

        // Check ak1 is not available from secondary index by using named method
        let r = Ak::load_by_sk(
            db.clone(),
            "EIMEIGHOH2GA5AEM4TAE6JIEROER0INGOOZEACAI".to_string(),
        )
        .await;
        assert!(r.is_err());

        // Check ak1 can't be loaded by primary key
        let r = Ak::load(db.clone(), &ak1.id).await;
        assert!(r.is_err());
        Ok(())
    }

    #[tokio::test]
    async fn test_03_update() -> Result<(), KvError> {
        let db = setup_database().await?;

        let ak1 = Ak {
            id: "4H2EKB28NOXPF6K40QOT".to_string(),
            sk: "EIMEIGHOH2GA5AEM4TAE6JIEROER0INGOOZEACAI".to_string(),
            state: "ACTIVE".to_string(),
            tags: None,
            marker: Ulid::from_str("01JRX2VBGFD15EH6H5H9AD5WC8").unwrap(),
            trusted: true,
            owner: "Bob".to_string(),
        };
        ak1.save(db.clone()).await?;

        // Save a second one
        let ak2 = Ak {
            id: "CKLTKHSLP99NZANMG9RK".to_string(),
            sk: "MNFKU0ZEYSUHUNVSZO8VMQEMLM8XBAYZQGUFMKAG".to_string(),
            state: "ACTIVE".to_string(),
            tags: None,
            marker: Ulid::from_str("01JSPW068HWN9RQH5WCTWXHM2W").unwrap(),
            trusted: true,
            owner: "Bob".to_string(),
        };
        ak2.save(db.clone()).await?;

        // Check Bob has now two ak
        let r = Ak::find_by_index(db.clone(), "owner", "Bob".to_string()).await?;
        assert!(r.len() == 2);

        // Update ak1
        let new_marker = Ulid::new();
        let ak_updated = Ak {
            state: "LOCKED".to_string(),
            marker: new_marker,
            owner: "Alice".to_string(),
            ..ak1.clone()
        };
        ak1.update(db.clone(), ak_updated.clone()).await?;

        // Check we can find ak1 by his new marker value and has the new state
        let r = Ak::load_by_marker(db.clone(), new_marker).await?;
        assert!(r == ak_updated && r.state == *"LOCKED");

        // Check Bob has only one remaining Ak
        let r = Ak::find_by_index(db.clone(), "owner", "Bob".to_string()).await?;
        assert!(r.len() == 1);

        // Check Alice has only one Ak with the good id
        let r = Ak::find_by_index(db.clone(), "owner", "Alice".to_string()).await?;
        assert!(r.len() == 1 && r.first().unwrap().id == *"4H2EKB28NOXPF6K40QOT");

        Ok(())
    }
}
