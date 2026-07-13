use rusqlite::Connection;

use crate::block::ClaimBlock;

pub fn save(claim: &ClaimBlock) -> Result<(), String> {
    let conn = Connection::open("lendlink.db");
    if conn.is_err() {
        return Err("Failed to open database".to_string());
    }
    let conn = conn.unwrap();

    let result = conn.execute(
        "CREATE TABLE IF NOT EXISTS blocks (
            previous_hash TEXT,
            hash TEXT NOT NULL PRIMARY KEY,
            issuer TEXT NOT NULL,
            lender TEXT NOT NULL,
            borrower TEXT NOT NULL,
            amount INTEGER NOT NULL,
            issued_at TEXT NOT NULL,
            issuer_signature TEXT NOT NULL
        );",
        [],
    );
    if result.is_err() {
        return Err("Failed to initiate SQL database".to_string());
    }

    if claim.issuer_signature.is_none() {
        return Err("Missing issuer signature".to_string());
    }

    let result = conn.execute(
        "INSERT INTO blocks VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        (
            claim.previous_hash.clone(),
            claim.hash.clone(),
            claim.issuer.to_string(),
            hex::encode(claim.lender.to_bytes()),
            hex::encode(claim.borrower.to_bytes()),
            claim.amount,
            claim.issued_at.clone(),
            hex::encode(claim.issuer_signature.unwrap().to_bytes()),
        ),
    );

    if result.is_err() {
        return Err("Failed to execute SQL query".to_string());
    }

    Ok(())
}
