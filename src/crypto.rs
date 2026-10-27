use crate::error::{Error, Result};
use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use chacha20poly1305::{
    aead::{Aead, KeyInit, Payload},
    XChaCha20Poly1305, XNonce,
};
use rand::{rngs::OsRng, RngCore};
use zeroize::Zeroizing;

pub fn random<const N: usize>() -> [u8; N] {
    let mut bytes = [0; N];
    OsRng.fill_bytes(&mut bytes);
    bytes
}
pub fn token() -> String {
    random::<32>().iter().map(|b| format!("{b:02x}")).collect()
}
pub fn hash_password(password: &str) -> Result<String> {
    Argon2::default()
        .hash_password(password.as_bytes(), &SaltString::generate(&mut OsRng))
        .map(|h| h.to_string())
        .map_err(|_| Error::Crypto)
}
pub fn verify(password: &str, hash: &str) -> bool {
    PasswordHash::new(hash).is_ok_and(|h| {
        Argon2::default()
            .verify_password(password.as_bytes(), &h)
            .is_ok()
    })
}
pub fn derive(password: &str, salt: &[u8]) -> Result<Zeroizing<[u8; 32]>> {
    let mut key = Zeroizing::new([0; 32]);
    Argon2::default()
        .hash_password_into(password.as_bytes(), salt, &mut *key)
        .map_err(|_| Error::Crypto)?;
    Ok(key)
}
pub fn encrypt(key: &[u8; 32], bytes: &[u8]) -> Result<Vec<u8>> {
    let nonce = random::<24>();
    let cipher = XChaCha20Poly1305::new(key.into());
    let sealed = cipher
        .encrypt(
            XNonce::from_slice(&nonce),
            Payload {
                msg: bytes,
                aad: b"prm:mental-vault:v1",
            },
        )
        .map_err(|_| Error::Crypto)?;
    Ok([nonce.to_vec(), sealed].concat())
}
pub fn decrypt(key: &[u8; 32], bytes: &[u8]) -> Result<Zeroizing<Vec<u8>>> {
    if bytes.len() < 40 {
        return Err(Error::Crypto);
    }
    XChaCha20Poly1305::new(key.into())
        .decrypt(
            XNonce::from_slice(&bytes[..24]),
            Payload {
                msg: &bytes[24..],
                aad: b"prm:mental-vault:v1",
            },
        )
        .map(Zeroizing::new)
        .map_err(|_| Error::Crypto)
}
