use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand_core::OsRng;
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
