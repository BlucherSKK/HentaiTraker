use argon2::{
    password_hash::{
        rand_core::OsRng,
        PasswordHash, PasswordHasher,
        PasswordVerifier, // <--- Добавьте этот трейт
        SaltString,
        Error as PasswordHashError
    },
    Argon2
};

pub fn hash_password(password: &str) -> Result<String, PasswordHashError> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    let password_hash = argon2.hash_password(password.as_bytes(), &salt)?;

    Ok(password_hash.to_string())
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    // Если формат хеша в БД будет битым, PasswordHash::new вернет Error.
    // Пока используем .ok(), чтобы просто вернуть false в случае ошибки парсинга.
    if let Ok(parsed_hash) = PasswordHash::new(hash) {
        return Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok();
    }
    false
}
