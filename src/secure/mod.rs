use argon2::{
    password_hash::{
        rand_core::OsRng,
        Error as PasswordHashError,
        PasswordHash, PasswordHasher,
        PasswordVerifier, SaltString,
    },
    Argon2,
};
use base64::{engine::general_purpose, Engine as _};
use sha2::{Sha256, Digest};
use aes_gcm::{Aes256Gcm, Key, Nonce, aead::{Aead, KeyInit}};

pub fn get_token(size_in_bytes: usize) -> String {
    let mut bytes = vec![0u8; size_in_bytes];
    rand::fill(&mut bytes);
    general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

pub fn hash_password(password: &str) -> Result<String, PasswordHashError> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
    .hash_password(password.as_bytes(), &salt)
    .map(|hash| hash.to_string())
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    PasswordHash::new(hash).map_or(false, |parsed_hash| {
        Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
    })
}

fn derive_key(material: &str) -> [u8; 32] {
    Sha256::digest(material.as_bytes()).into()
}

/// Шифрует данные ключом key_material через AES-256-GCM.
/// Возвращает nonce(12 байт) || ciphertext || tag — всё в одном буфере.
/// @param key_material - строка токена, из которой выводится ключ через SHA-256
/// @param plaintext    - открытые данные для шифрования
pub fn encrypt(key_material: &str, plaintext: &[u8]) -> Vec<u8> {
    let key_bytes = derive_key(key_material);
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);
    let mut nonce_bytes = [0u8; 12];
    rand::fill(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher.encrypt(nonce, plaintext).expect("encrypt failed");
    let mut out = nonce_bytes.to_vec();
    out.extend(ciphertext);
    out
}

/// Дешифрует данные ключом key_material через AES-256-GCM.
/// Ожидает формат nonce(12 байт) || ciphertext || tag.
/// Возвращает None при ошибке дешифровки или неверном формате.
/// @param key_material - строка токена, из которой выводится ключ через SHA-256
/// @param data         - зашифрованный буфер
pub fn decrypt(key_material: &str, data: &[u8]) -> Option<Vec<u8>> {
    if data.len() < 12 { return None; }
    let (nonce_bytes, ciphertext) = data.split_at(12);
    let key_bytes = derive_key(key_material);
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);
    cipher.decrypt(Nonce::from_slice(nonce_bytes), ciphertext).ok()
}

/// Возвращает SHA-256 хеш токена в hex-кодировке.
/// Используется в состоянии PrivateOnly для верификации клиента без раскрытия ключа.
/// @param token - строка токена для хеширования
pub fn token_hash(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}
