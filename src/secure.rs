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
