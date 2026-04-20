use pqcrypto_kyber::kyber1024::*;
use pqcrypto_traits::kem::{Ciphertext as _, PublicKey as _, SecretKey as _, SharedSecret as _};
use base64::{Engine as _, engine::general_purpose};

pub struct KeyPair {
    pub public_key: String,
    pub secret_key: String,
}

pub fn generate_quantum_keys() -> KeyPair {
    let (pk, sk) = keypair();
    KeyPair {
        public_key: general_purpose::STANDARD.encode(pk.as_bytes()),
        secret_key: general_purpose::STANDARD.encode(sk.as_bytes()),
    }
}

pub fn encrypt_with_kyber(pk_base64: &str) -> (String, String) {
    let pk_bytes = general_purpose::STANDARD.decode(pk_base64).unwrap();
    let pk = PublicKey::from_bytes(&pk_bytes).unwrap();
    let (ss, ct) = encapsulate(&pk);
    (
        general_purpose::STANDARD.encode(ss.as_bytes()),
        general_purpose::STANDARD.encode(ct.as_bytes()),
    )
}

pub fn decrypt_with_kyber(ct_base64: &str, sk_base64: &str) -> String {
    let ct_bytes = general_purpose::STANDARD.decode(ct_base64).unwrap();
    let sk_bytes = general_purpose::STANDARD.decode(sk_base64).unwrap();
    let ct = Ciphertext::from_bytes(&ct_bytes).unwrap();
    let sk = SecretKey::from_bytes(&sk_bytes).unwrap();
    let ss = decapsulate(&ct, &sk);
    general_purpose::STANDARD.encode(ss.as_bytes())
}
