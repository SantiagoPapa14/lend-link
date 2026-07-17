use rusqlite::Connection;

use crate::{
    block::{ClaimBlock, Hash, Issuer},
    crypto,
};

pub enum StorageType {
    Sqlite,
    Memory,
}

pub trait BlockStorage: Sized {
    fn init() -> Result<Self, String>;
    fn save(&self, claim: &ClaimBlock, chain_id: Hash) -> Result<(), String>;
    fn load(&self) -> Result<Vec<ClaimBlock>, String>;
}

pub struct SqliteStorage {
    conn: Connection,
}

impl BlockStorage for SqliteStorage {
    fn init() -> Result<SqliteStorage, String> {
        let conn = init_db(StorageType::Sqlite).map_err(|e| e.to_string())?;
        Ok(SqliteStorage { conn })
    }
    fn save(&self, claim: &ClaimBlock, chain_id: Hash) -> Result<(), String> {
        insert_block(&self.conn, claim, chain_id)
    }
    fn load(&self) -> Result<Vec<ClaimBlock>, String> {
        return get_all_claims(&self.conn);
    }
}

pub struct MemoryStorage {
    conn: Connection,
}

impl BlockStorage for MemoryStorage {
    fn init() -> Result<MemoryStorage, String> {
        let conn = init_db(StorageType::Memory).map_err(|e| e.to_string())?;
        Ok(MemoryStorage { conn })
    }
    fn save(&self, claim: &ClaimBlock, chain_id: Hash) -> Result<(), String> {
        insert_block(&self.conn, claim, chain_id)
    }
    fn load(&self) -> Result<Vec<ClaimBlock>, String> {
        return get_all_claims(&self.conn);
    }
}

fn init_db(storage_type: StorageType) -> Result<Connection, String> {
    let conn = match storage_type {
        StorageType::Sqlite => Connection::open("lendlink.db").map_err(|e| e.to_string())?,
        StorageType::Memory => Connection::open_in_memory().map_err(|e| e.to_string())?,
    };

    let _ = conn
        .execute(
            "CREATE TABLE IF NOT EXISTS blocks (
            previous_hash TEXT,
            hash TEXT NOT NULL PRIMARY KEY,
            issuer TEXT NOT NULL,
            lender BLOB NOT NULL CHECK(length(lender) = 32),
            borrower BLOB NOT NULL CHECK(length(borrower) = 32),
            amount INTEGER NOT NULL,
            issued_at TEXT NOT NULL,
            issuer_signature BLOB NOT NULL CHECK(length(issuer_signature) = 64),
            chain_id TEXT NOT NULL
        );",
            [],
        )
        .map_err(|e| e.to_string())?;

    Ok(conn)
}

pub fn insert_block(conn: &Connection, claim: &ClaimBlock, chain_id: Hash) -> Result<(), String> {
    let _ = conn
        .execute(
            "
    INSERT INTO blocks (
        previous_hash,
        hash,
        issuer,
        lender,
        borrower,
        amount,
        issued_at,
        issuer_signature,
        chain_id
    )
    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
    ON CONFLICT(hash)
    DO UPDATE SET
        chain_id = excluded.chain_id
    ",
            (
                claim.previous_hash.clone(),
                claim.hash.clone(),
                claim.issuer.to_string(),
                claim.lender.to_bytes(),
                claim.borrower.to_bytes(),
                claim.amount,
                claim.issued_at.to_string(),
                claim
                    .issuer_signature
                    .ok_or_else(|| "Issuer signature is missing".to_string())?
                    .to_bytes(),
                chain_id,
            ),
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn get_all_claims(conn: &Connection) -> Result<Vec<ClaimBlock>, String> {
    let mut stmt = conn
        .prepare("SELECT * FROM blocks")
        .map_err(|e| e.to_string())?;

    let claims: Result<Vec<ClaimBlock>, rusqlite::Error> = stmt
        .query_map((), |row| {
            Ok(ClaimBlock {
                previous_hash: row.get(0).unwrap_or(None),
                hash: row.get(1)?,
                issuer: row.get(2)?,
                lender: crypto::read_verifying_key(row, 3)?,
                borrower: crypto::read_verifying_key(row, 4)?,
                amount: row.get(5)?,
                issued_at: row.get(6)?,
                issuer_signature: crypto::read_optional_signature(row, 7)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect();

    let claims = claims.map_err(|e| e.to_string())?;
    return Ok(claims);
}

#[cfg(test)]
mod tests {
    use crate::{
        block::{ClaimBlock, Issuer},
        crypto,
    };

    use super::{BlockStorage, MemoryStorage, StorageType, init_db, insert_block};

    fn stored_claim(
        hash: &str,
        previous_hash: Option<&str>,
        issuer: Issuer,
        amount: i32,
    ) -> ClaimBlock {
        let (signing_key, lender) = crypto::generate_keys();
        let (_, borrower) = crypto::generate_keys();
        let signature = crypto::sign_message(&signing_key, hash.as_bytes());

        ClaimBlock {
            previous_hash: previous_hash.map(|value| value.to_string()),
            hash: hash.to_string(),
            issuer,
            lender,
            borrower,
            amount,
            issued_at: "2026-07-10T00:00:00Z".to_string(),
            issuer_signature: Some(signature),
        }
    }

    #[test]
    fn inserting_an_existing_block_updates_its_chain_id() {
        let conn = init_db(StorageType::Memory).unwrap();
        let (signing_key, lender) = crypto::generate_keys();
        let (_, borrower) = crypto::generate_keys();
        let signature = crypto::sign_message(&signing_key, b"stored claim");
        let claim = ClaimBlock {
            previous_hash: Some("missing-parent".to_string()),
            hash: "stored-claim".to_string(),
            issuer: Issuer::Lender,
            lender,
            borrower,
            amount: 10,
            issued_at: "2026-07-10T00:00:00Z".to_string(),
            issuer_signature: Some(signature),
        };

        insert_block(&conn, &claim, "orphan".to_string()).unwrap();
        insert_block(&conn, &claim, "genesis-hash".to_string()).unwrap();

        let stored_chain_id: String = conn
            .query_row(
                "SELECT chain_id FROM blocks WHERE hash = ?1",
                [&claim.hash],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(stored_chain_id, "genesis-hash");
    }

    #[test]
    fn load_returns_saved_claims_from_memory_storage() {
        let storage = MemoryStorage::init().unwrap();
        let claim = stored_claim("claim-1", Some("genesis"), Issuer::Lender, 10);

        storage.save(&claim, "genesis-hash".to_string()).unwrap();

        let loaded = storage.load().unwrap();
        assert_eq!(loaded.len(), 1);

        let loaded_claim = &loaded[0];
        assert_eq!(loaded_claim.previous_hash, claim.previous_hash);
        assert_eq!(loaded_claim.hash, claim.hash);
        assert_eq!(loaded_claim.issuer.to_string(), claim.issuer.to_string());
        assert_eq!(loaded_claim.lender.to_bytes(), claim.lender.to_bytes());
        assert_eq!(loaded_claim.borrower.to_bytes(), claim.borrower.to_bytes());
        assert_eq!(loaded_claim.amount, claim.amount);
        assert_eq!(loaded_claim.issued_at, claim.issued_at);
        assert_eq!(
            loaded_claim.issuer_signature.as_ref().unwrap().to_bytes(),
            claim.issuer_signature.as_ref().unwrap().to_bytes()
        );
    }

    #[test]
    fn load_returns_every_saved_claim() {
        let storage = MemoryStorage::init().unwrap();
        let first = stored_claim("claim-1", Some("genesis"), Issuer::Borrower, 10);
        let second = stored_claim("claim-2", Some("claim-1"), Issuer::Lender, 25);

        storage.save(&first, "genesis-hash".to_string()).unwrap();
        storage.save(&second, "genesis-hash".to_string()).unwrap();

        let loaded = storage.load().unwrap();
        let loaded_hashes: Vec<_> = loaded.iter().map(|claim| claim.hash.as_str()).collect();

        assert_eq!(loaded.len(), 2);
        assert!(loaded_hashes.contains(&"claim-1"));
        assert!(loaded_hashes.contains(&"claim-2"));
    }
}
