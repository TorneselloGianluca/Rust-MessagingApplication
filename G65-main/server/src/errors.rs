use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("dati non validi: {0}")]
    Validation(String),
    #[error("utente già esistente")]
    UserExists,
    #[error("credenziali errate")]
    BadCredentials,
    #[error("db error: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("crypto error: {0}")]
    Crypto(#[from] argon2::password_hash::Error),
    #[error("serde error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub type AppResult<T> = Result<T, AppError>;
