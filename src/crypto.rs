use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand_core::OsRng;
use rusqlite::types::Type;
use sha3::{Digest, Sha3_256};

pub fn hash(input: &str) -> String {
    let digest = Sha3_256::digest(input.as_bytes());
    hex::encode(digest)
}

pub fn generate_keys() -> (SigningKey, VerifyingKey) {
    let mut rng = OsRng;
    let signing_key = SigningKey::generate(&mut rng);
    let verifying_key = signing_key.verifying_key();

    (signing_key, verifying_key)
}

pub fn sign_message(signing_key: &SigningKey, message: &[u8]) -> Signature {
    signing_key.sign(message)
}

pub fn verify_signature(
    verifying_key: &VerifyingKey,
    message: &[u8],
    signature: &Signature,
) -> bool {
    verifying_key.verify(message, signature).is_ok()
}

pub fn read_verifying_key(row: &rusqlite::Row<'_>, index: usize) -> rusqlite::Result<VerifyingKey> {
    let bytes: Vec<u8> = row.get(index)?;

    let bytes: [u8; 32] = bytes.try_into().map_err(|bytes: Vec<u8>| {
        rusqlite::Error::FromSqlConversionFailure(
            index,
            Type::Blob,
            format!(
                "invalid Ed25519 verifying key length: expected 32, got {}",
                bytes.len()
            )
            .into(),
        )
    })?;

    VerifyingKey::from_bytes(&bytes).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(index, Type::Blob, Box::new(error))
    })
}

pub fn read_optional_signature(
    row: &rusqlite::Row<'_>,
    index: usize,
) -> rusqlite::Result<Option<Signature>> {
    let bytes: Option<Vec<u8>> = row.get(index)?;

    bytes
        .map(|bytes| {
            let bytes: [u8; 64] = bytes.try_into().map_err(|bytes: Vec<u8>| {
                rusqlite::Error::FromSqlConversionFailure(
                    index,
                    Type::Blob,
                    format!(
                        "invalid Ed25519 signature length: expected 64, got {}",
                        bytes.len()
                    )
                    .into(),
                )
            })?;

            Ok(Signature::from_bytes(&bytes))
        })
        .transpose()
}
