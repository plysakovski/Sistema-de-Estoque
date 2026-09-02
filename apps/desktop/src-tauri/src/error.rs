use serde::ser::{Serialize, Serializer};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Erro de banco de dados: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("Erro de entrada/saída: {0}")]
    Io(#[from] std::io::Error),
    #[error("Dados inválidos: {0}")]
    Validation(String),
    #[error("Registro não encontrado: {0}")]
    NotFound(String),
    #[error("Autenticação necessária: {0}")]
    Unauthorized(String),
    #[error("Acesso negado: {0}")]
    Forbidden(String),
    #[error("Conflito: {0}")]
    Conflict(String),
    #[error("Falha de segurança: {0}")]
    Security(String),
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
