use rusqlite::Connection;

use crate::block::{ClaimBlock, Hash};

pub enum StorageType {
    Sqlite,
    Memory,
}

pub trait BlockStorage: Sized {
    fn init() -> Result<Self, String>;
    fn save(&self, claim: &ClaimBlock, chain_id: Hash) -> Result<(), String>;
    // fn load(...)...
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
            lender TEXT NOT NULL,
            borrower TEXT NOT NULL,
            amount INTEGER NOT NULL,
            issued_at TEXT NOT NULL,
            issuer_signature TEXT NOT NULL,
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
                hex::encode(claim.lender.to_bytes()),
                hex::encode(claim.borrower.to_bytes()),
                claim.amount,
                claim.issued_at.to_string(),
                hex::encode(
                    claim
                        .issuer_signature
                        .ok_or_else(|| "Issuer signature is missing".to_string())?
                        .to_bytes(),
                ),
                chain_id,
            ),
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{
        block::{ClaimBlock, Issuer},
        crypto,
    };

    use super::{StorageType, init_db, insert_block};

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
}
