use crate::model::AppData;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, time::Instant};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub username: String,
    pub password_hash: String,
    pub data: AppData,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AccountsDb {
    pub accounts: BTreeMap<String, Account>,
}

pub struct AuthState {
    pub db: AccountsDb,
    pub active: Option<String>,
    pub failed_logins: u8,
    pub retry_after: Option<Instant>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountStatus {
    pub has_accounts: bool,
    pub active_username: Option<String>,
}

pub fn account_key(username: &str) -> String {
    username.to_ascii_lowercase()
}

pub fn validate_credentials(username: &str, password: &str) -> Result<(), String> {
    if !(3..=32).contains(&username.len())
        || !username
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
    {
        return Err("Username must be 3–32 letters, numbers, hyphens, or underscores".into());
    }
    if !(8..=128).contains(&password.chars().count()) {
        return Err("Password must be 8–128 characters".into());
    }
    Ok(())
}

pub fn hash_password(password: &str) -> Result<String, String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|_| "Could not secure the password".into())
}

pub fn verify_password(password: &str, encoded: &str) -> bool {
    PasswordHash::new(encoded).ok().is_some_and(|hash| {
        Argon2::default()
            .verify_password(password.as_bytes(), &hash)
            .is_ok()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn credentials_are_bounded() {
        assert!(validate_credentials("valid_user", "long-enough").is_ok());
        assert!(validate_credentials("../bad", "long-enough").is_err());
        assert!(validate_credentials("ok_user", "short").is_err());
        assert!(validate_credentials("ok_user", "éééé").is_err());
        assert!(validate_credentials("ok_user", "🔐🔐🔐🔐🔐🔐🔐🔐").is_ok());
    }
    #[test]
    fn passwords_are_salted_and_verified() {
        let first = hash_password("correct horse").unwrap();
        let second = hash_password("correct horse").unwrap();
        assert_ne!(first, second);
        assert!(!first.contains("correct horse"));
        assert!(verify_password("correct horse", &first));
        assert!(!verify_password("wrong password", &first));
    }
}
