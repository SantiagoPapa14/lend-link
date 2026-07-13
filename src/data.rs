use rusqlite::{Connection, Result};

pub fn save(claim: &ClaimBlock) {
    let conn = Connection::open("lendlink.db")?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS blocks (
            previous_hash TEXT
            hash TEXT NOT NULL PRIMARY KEY
            issuer TEXT NOT NULL
            lender TEXT NOT NULL
            borrower TEXT NOT NULL
            amount INTEGER P NOT NULL
            issued_at TEXT NOT NULL
            issuer_signature TEXT NOT NULL
        )",
        [],
    )?;

    let conn = Connection::open("example.db")?;

    conn.execute(
        "INSERT INTO blocks VALUES (?1, ?2, ?3, 4?, 5?, 6?, 7?, 8?)",
        (claim.previous_hash.clone(), 25),
    )?;

    Ok(())
}

pub struct ClaimBlock {
    pub previous_hash: Option<Hash>,

    #[serde(skip_serializing)]
    pub hash: Hash,

    pub issuer: Issuer,
    pub lender: Address,
    pub borrower: Address,
    pub amount: i32,
    pub issued_at: String,

    #[serde(skip_serializing)]
    pub issuer_signature: Option<Signature>,
}
