use rusqlite::Connection;

use crate::block::{ClaimBlock, Hash};

pub enum StorageType {
    Test,
    Sqlite,
}

fn init_db(storage_type: StorageType) -> Result<Connection, String> {
    let conn = match storage_type {
        StorageType::Sqlite => Connection::open("lendlink.db").map_err(|e| e.to_string())?,
        StorageType::Test => Connection::open_in_memory().map_err(|e| e.to_string())?,
    };

    let _ = conn
        .execute(
            "create table if not exists blocks (
            previous_hash text,
            hash text not null primary key,
            issuer text not null,
            lender text not null,
            borrower text not null,
            amount integer not null,
            issued_at text not null,
            issuer_signature text not null,
            chain_id text not null
        );",
            [],
        )
        .map_err(|e| e.to_string())?;

    Ok(conn)
}

pub fn save_test(claim: &ClaimBlock, chain_id: Hash) -> Result<(), String> {
    let conn = init_db(StorageType::Sqlite).map_err(|e| e.to_string())?;
    insert_block(&conn, claim, chain_id).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn save(claim: &ClaimBlock, chain_id: Hash) -> Result<(), String> {
    let conn = init_db(StorageType::Sqlite).map_err(|e| e.to_string())?;
    insert_block(&conn, claim, chain_id).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn insert_block(conn: &Connection, claim: &ClaimBlock, chain_id: Hash) -> Result<(), String> {
    let _ = conn
        .execute(
            "INSERT INTO blocks VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            (
                claim.previous_hash.clone(),
                claim.hash.clone(),
                claim.issuer.to_string(),
                hex::encode(claim.lender.to_bytes()),
                hex::encode(claim.borrower.to_bytes()),
                claim.amount,
                claim.issued_at.clone(),
                hex::encode(
                    claim
                        .issuer_signature
                        .ok_or_else(|| "Issuer signature is missing".to_string())?
                        .to_bytes(),
                ),
                chain_id.clone(),
            ),
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}
