/// tests must be launch with:
/// `cargo test -- tests --nocapture --test-threads=1`
/// to avoid any db conflict between them
#[cfg(test)]
mod tests {
    use fdb_derive::FdbStore;
    use fdb_trait::{FdbStore, KvError, RangeQuery};
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

        // Ak::find_by_unique_index_range(db, index_name, start, stop)

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

    #[tokio::test]
    async fn test_04_range_uniq_index_query() -> Result<(), KvError> {
        let db = setup_database().await?;

        let ak1 = Ak {
            id: "".to_string(),
            sk: "".to_string(),
            state: "ACTIVE".to_string(),
            tags: None,
            marker: Ulid::from_str("01JRX2VBGFD15EH6H5H9AD5WC8").unwrap(),
            trusted: true,
            owner: "Bob".to_string(),
        };

        let aks: Vec<Ak> = (1..20)
            .map(|i| Ak {
                id: format!("id-{:?}", i),
                sk: format!("sk-{:?}", i),
                marker: ulid::Ulid::new(),
                ..ak1.clone()
            })
            .collect();

        for ele in &aks {
            ele.save(db.clone()).await?;
        }
        println!("saved...");
        aks.iter()
            .for_each(|ak| println!("id: {:?}, sk: {:?}", ak.id, ak.sk,));

        // test by getting the first 5 items
        let range =
            Ak::find_by_unique_index_range_sk(db.clone(), RangeQuery::NFirstResults(5), false)
                .await?;

        println!("Range: {:#?}", range);
        assert!(range.len() == 5);
        assert!(range.last().unwrap().sk == *"sk-5");

        // test by getting in range (between start and stop)
        let range = Ak::find_by_unique_index_range_sk(
            db.clone(),
            RangeQuery::StartAndStop("sk-4".to_owned(), "sk-9".to_owned()),
            false,
        )
        .await?;
        println!("Range: {:#?}", range);

        assert!(range.len() == 5);

        // test by getting all
        let range = Ak::find_by_unique_index_range_sk(db.clone(), RangeQuery::All, false).await?;
        println!("Range: {:#?}", range);
        assert!(range.len() == 19);
        assert!(range.last() == aks.last());

        // test getting 10 result from "sk-5"
        let range = Ak::find_by_unique_index_range_sk(
            db.clone(),
            RangeQuery::StartAndNbResult("sk-5".to_owned(), 10),
            false,
        )
        .await?;
        println!("Range: {:#?}", range);
        assert!(range.len() == 10);
        Ok(())
    }

    #[tokio::test]
    async fn test_05_range_primary_index_query() -> Result<(), KvError> {
        let db = setup_database().await?;

        let ak1 = Ak {
            id: "".to_string(),
            sk: "".to_string(),
            state: "ACTIVE".to_string(),
            tags: None,
            marker: Ulid::from_str("01JRX2VBGFD15EH6H5H9AD5WC8").unwrap(),
            trusted: true,
            owner: "Bob".to_string(),
        };

        let aks: Vec<Ak> = (1..20)
            .map(|i| Ak {
                id: format!("id-{:?}", i),
                sk: format!("sk-{:?}", i),
                marker: ulid::Ulid::new(),
                ..ak1.clone()
            })
            .collect();

        for ele in &aks {
            ele.save(db.clone()).await?;
        }
        println!("saved...");

        let all = Ak::load_by_primary_range(db.clone(), RangeQuery::All).await?;
        assert!(all.len() == 19);

        let range1 = Ak::load_by_primary_range(
            db.clone(),
            RangeQuery::StartAndStop("id-3".to_owned(), "id-6".to_owned()),
        )
        .await?;
        assert!(range1.len() == 3);

        Ok(())
    }
}
