use ed25519_dalek::{Signature, VerifyingKey};
use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ValueRef};
use serde::{Deserialize, Serialize};

use crate::crypto;

pub type Hash = String;
pub type Address = VerifyingKey;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Issuer {
    Borrower,
    Lender,
    Genesis,
}

impl Issuer {
    pub fn to_string(&self) -> String {
        match self {
            Issuer::Lender => String::from("Lender"),
            Issuer::Borrower => String::from("Borrower"),
            Issuer::Genesis => String::from("Genesis"),
        }
    }
}

impl FromSql for Issuer {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        match value.as_str()? {
            "Lender" => Ok(Issuer::Lender),
            "Borrower" => Ok(Issuer::Borrower),
            "Genesis" => Ok(Issuer::Genesis),
            _ => Err(FromSqlError::InvalidType),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
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

impl ClaimBlock {
    pub fn calculate_hash(&self) -> Hash {
        let serial = serde_json::to_string(self).unwrap();
        crypto::hash(&serial)
    }

    pub fn validate(&self) -> bool {
        let new_hash = self.calculate_hash();
        if new_hash != self.hash {
            return false;
        }

        let signer = match self.issuer {
            Issuer::Lender => self.lender,
            Issuer::Borrower => self.borrower,
            Issuer::Genesis => {
                if self.lender == self.borrower {
                    self.lender
                } else {
                    return false;
                }
            }
        };

        let Some(signature) = self.issuer_signature.as_ref() else {
            return false;
        };

        return crypto::verify_signature(&signer, new_hash.as_bytes(), signature);
    }
}

#[cfg(test)]
mod tests {
    use crate::crypto;

    use super::{ClaimBlock, Issuer};

    fn claim_with_hash(
        hash: &str,
        amount: i32,
        lender_public: crate::block::Address,
        borrower_public: crate::block::Address,
        issuer: Issuer,
    ) -> ClaimBlock {
        ClaimBlock {
            previous_hash: Some("phash".to_string()),
            hash: hash.to_string(),
            issuer: issuer,
            lender: lender_public,
            borrower: borrower_public,
            amount,
            issued_at: "2026-07-09T00:00:00Z".to_string(),
            issuer_signature: None,
        }
    }

    #[test]
    fn hash_changes_when_claim_content_changes() {
        let (_, lender_public) = crypto::generate_keys();
        let (_, borrower_public) = crypto::generate_keys();
        let a = claim_with_hash(
            "stored-a",
            10,
            lender_public,
            borrower_public,
            Issuer::Lender,
        );
        let b = claim_with_hash(
            "stored-a",
            11,
            lender_public,
            borrower_public,
            Issuer::Lender,
        );

        assert_ne!(a.calculate_hash(), b.calculate_hash());
    }

    #[test]
    fn hash_should_not_depend_on_the_stored_hash_field() {
        let (_, lender_public) = crypto::generate_keys();
        let (_, borrower_public) = crypto::generate_keys();
        let a = claim_with_hash(
            "stored-a",
            10,
            lender_public,
            borrower_public,
            Issuer::Borrower,
        );
        let b = claim_with_hash(
            "stored-b",
            10,
            lender_public,
            borrower_public,
            Issuer::Borrower,
        );

        assert_eq!(a.calculate_hash(), b.calculate_hash());
    }

    #[test]
    fn signed_claim_hash_verifies() {
        let (signing_key, verifying_key) = crypto::generate_keys();
        let (_, lender_public) = crypto::generate_keys();
        let (_, borrower_public) = crypto::generate_keys();
        let claim = claim_with_hash(
            "stored-a",
            10,
            lender_public,
            borrower_public,
            Issuer::Lender,
        );
        let hash = claim.calculate_hash();
        let signature = crypto::sign_message(&signing_key, hash.as_bytes());
        let signed_claim = ClaimBlock {
            issuer_signature: Some(signature),
            hash,
            ..claim
        };

        assert!(crypto::verify_signature(
            &verifying_key,
            signed_claim.calculate_hash().as_bytes(),
            signed_claim.issuer_signature.as_ref().unwrap()
        ));
    }

    #[test]
    fn changing_a_claim_after_signing_breaks_verification() {
        let (signing_key, verifying_key) = crypto::generate_keys();
        let (_, lender_public) = crypto::generate_keys();
        let (_, borrower_public) = crypto::generate_keys();
        let claim = claim_with_hash(
            "stored-a",
            10,
            lender_public,
            borrower_public,
            Issuer::Lender,
        );
        let hash = claim.calculate_hash();
        let signature = crypto::sign_message(&signing_key, hash.as_bytes());

        let mut tampered_claim = claim;
        tampered_claim.amount = 11;
        let tampered_hash = tampered_claim.calculate_hash();

        assert_ne!(hash, tampered_hash);
        assert!(!crypto::verify_signature(
            &verifying_key,
            tampered_hash.as_bytes(),
            &signature
        ));
    }

    #[test]
    fn validate_accepts_a_correctly_signed_claim() {
        let (signing_key, borrower_public) = crypto::generate_keys();
        let (_, lender_public) = crypto::generate_keys();
        let claim = claim_with_hash(
            "stored-a",
            10,
            lender_public,
            borrower_public,
            Issuer::Borrower,
        );
        let hash = claim.calculate_hash();
        let signature = crypto::sign_message(&signing_key, hash.as_bytes());

        let claim = ClaimBlock {
            hash,
            issuer_signature: Some(signature),
            ..claim
        };

        assert!(claim.validate());
    }

    #[test]
    fn validate_rejects_missing_signature() {
        let (_, lender_public) = crypto::generate_keys();
        let (_, borrower_public) = crypto::generate_keys();
        let mut claim = claim_with_hash(
            "stored-a",
            10,
            lender_public,
            borrower_public,
            Issuer::Lender,
        );
        claim.hash = claim.calculate_hash();

        assert!(!claim.validate());
    }
}
