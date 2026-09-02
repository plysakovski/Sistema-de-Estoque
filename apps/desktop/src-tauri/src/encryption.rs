#[cfg(not(test))]
use keyring::{Entry, Error as KeyringError};
use rand_core::{OsRng, RngCore};
use rusqlite::{Connection, OpenFlags};
use zeroize::Zeroizing;

use crate::error::{AppError, AppResult};

#[cfg(not(test))]
const KEYRING_SERVICE: &str = "br.com.stockmanager.desktop";
#[cfg(not(test))]
const KEYRING_ACCOUNT: &str = "database-master-key-v1";
const KEY_BYTES: usize = 32;

pub struct StorageKey {
    value: Zeroizing<String>,
}

impl StorageKey {
    #[cfg(not(test))]
    pub fn load() -> AppResult<Option<Self>> {
        let entry = Entry::new(KEYRING_SERVICE, KEYRING_ACCOUNT)
            .map_err(|error| keyring_failure("acessar", error))?;
        match entry.get_password() {
            Ok(value) => Self::from_hex(value).map(Some),
            Err(KeyringError::NoEntry) => Ok(None),
            Err(error) => Err(keyring_failure("ler", error)),
        }
    }

    #[cfg(not(test))]
    pub fn persist(&self) -> AppResult<()> {
        Entry::new(KEYRING_SERVICE, KEYRING_ACCOUNT)
            .map_err(|error| keyring_failure("acessar", error))?
            .set_password(self.expose())
            .map_err(|error| keyring_failure("salvar", error))
    }

    pub fn expose(&self) -> &str {
        self.value.as_str()
    }

    pub(crate) fn generate() -> Self {
        let mut bytes = Zeroizing::new([0_u8; KEY_BYTES]);
        OsRng.fill_bytes(bytes.as_mut());
        let value = bytes
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        Self {
            value: Zeroizing::new(value),
        }
    }

    pub(crate) fn from_hex(value: String) -> AppResult<Self> {
        let valid =
            value.len() == KEY_BYTES * 2 && value.bytes().all(|byte| byte.is_ascii_hexdigit());
        if !valid {
            return Err(AppError::Security(
                "A chave protegida do banco possui formato inválido".into(),
            ));
        }
        Ok(Self {
            value: Zeroizing::new(value.to_ascii_lowercase()),
        })
    }
}

pub fn open_encrypted(path: &std::path::Path, key: &StorageKey) -> AppResult<Connection> {
    let connection = Connection::open(path)?;
    unlock(&connection, key)?;
    verify_unlocked(&connection)?;
    Ok(connection)
}

pub fn open_encrypted_read_only(path: &std::path::Path, key: &StorageKey) -> AppResult<Connection> {
    let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    unlock(&connection, key)?;
    verify_unlocked(&connection)?;
    Ok(connection)
}

pub fn unlock(connection: &Connection, key: &StorageKey) -> AppResult<()> {
    let key_literal = format!("x'{}'", key.expose());
    connection.pragma_update(None, "key", key_literal)?;
    connection.execute_batch("PRAGMA foreign_keys = ON; PRAGMA busy_timeout = 5000;")?;
    Ok(())
}

pub fn verify_unlocked(connection: &Connection) -> AppResult<()> {
    let cipher_version = connection
        .query_row("PRAGMA cipher_version", [], |row| row.get::<_, String>(0))
        .map_err(|_| AppError::Security("SQLCipher não está disponível nesta instalação".into()))?;
    if cipher_version.trim().is_empty() {
        return Err(AppError::Security(
            "SQLCipher não está disponível nesta instalação".into(),
        ));
    }
    connection
        .query_row("SELECT COUNT(*) FROM sqlite_master", [], |row| row.get::<_, i64>(0))
        .map_err(|_| {
            AppError::Security(
                "Não foi possível desbloquear o banco. A chave protegida pode estar ausente ou incorreta"
                    .into(),
            )
        })?;
    Ok(())
}

#[cfg(not(test))]
fn keyring_failure(operation: &str, error: KeyringError) -> AppError {
    AppError::Security(format!(
        "Não foi possível {operation} a chave no cofre de credenciais do Windows: {error}"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_key_has_256_bits_in_hex() {
        let key = StorageKey::generate();
        assert_eq!(key.expose().len(), 64);
        assert!(key.expose().bytes().all(|byte| byte.is_ascii_hexdigit()));
    }

    #[test]
    fn rejects_malformed_key() {
        assert!(StorageKey::from_hex("not-a-key".into()).is_err());
    }
}
