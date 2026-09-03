use std::time::Duration;

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Algorithm, Argon2, Params, Version,
};
use chrono::{DateTime, Utc};
use parking_lot::Mutex;
use zeroize::Zeroizing;

use crate::{
    database::Database,
    error::{AppError, AppResult},
    models::{
        AuthStatus, AuthenticatedUser, BootstrapAdminInput, CreateUserInput, LoginInput,
        ResetUserPasswordInput, UpdateUserInput, UserRole, UserView,
    },
};

const IDLE_TIMEOUT: Duration = Duration::from_secs(15 * 60);
const ABSOLUTE_TIMEOUT: Duration = Duration::from_secs(8 * 60 * 60);
const PASSWORD_MIN_CHARS: usize = 12;
const PASSWORD_MAX_CHARS: usize = 128;

#[derive(Debug, Clone, Copy)]
pub enum Permission {
    ReadInventory,
    RequestAdjustment,
    RequestReplenishment,
    ReceiveReplenishmentTransfer,
    RecordMovement,
    UpdateOperationalAssetStatus,
    ReportAssetIncident,
    ManageInventory,
    ReviewAdjustment,
    ReviewReplenishment,
    DispatchReplenishmentTransfer,
    ReviewAssetIncident,
    ViewAudit,
    ManageOperations,
    ManageLocations,
    RestoreBackup,
    ManageUsers,
    ManageEncryption,
}

#[derive(Debug, Clone)]
struct Session {
    user: AuthenticatedUser,
    session_version: i64,
    created_at: DateTime<Utc>,
    last_activity_at: DateTime<Utc>,
}

pub struct AuthService {
    session: Mutex<Option<Session>>,
    dummy_password_hash: String,
}

impl AuthService {
    pub fn new() -> AppResult<Self> {
        Ok(Self {
            session: Mutex::new(None),
            dummy_password_hash: hash_password("stockmanager-dummy-password")?,
        })
    }

    pub fn status(&self, database: &Database) -> AppResult<AuthStatus> {
        let setup_required = database.auth_setup_required()?;
        let user = if setup_required {
            self.clear_session();
            None
        } else {
            self.current_user(database).ok()
        };
        Ok(AuthStatus {
            setup_required,
            user,
        })
    }

    pub fn bootstrap(
        &self,
        database: &Database,
        input: BootstrapAdminInput,
    ) -> AppResult<AuthenticatedUser> {
        validate_username(&input.username)?;
        validate_display_name(&input.display_name)?;
        validate_password(&input.password, &input.username)?;
        let password = Zeroizing::new(input.password);
        let password_hash = hash_password(password.as_str())?;
        let (user, session_version) = database.bootstrap_admin(
            input.username.trim(),
            input.display_name.trim(),
            &password_hash,
        )?;
        self.start_session(user.clone(), session_version);
        Ok(user)
    }

    pub fn login(&self, database: &Database, input: LoginInput) -> AppResult<AuthenticatedUser> {
        let password = Zeroizing::new(input.password);
        let record = database.find_user_for_login(input.username.trim())?;
        let hash = record
            .as_ref()
            .map(|record| record.password_hash.as_str())
            .unwrap_or(self.dummy_password_hash.as_str());
        let valid = verify_password(password.as_str(), hash);

        let Some(record) = record else {
            return Err(invalid_credentials());
        };
        if record
            .locked_until
            .as_deref()
            .and_then(parse_timestamp)
            .is_some_and(|until| until > Utc::now())
        {
            return Err(invalid_credentials());
        }
        if !valid {
            database.record_login_failure(&record.user.id, record.failed_login_attempts)?;
            return Err(invalid_credentials());
        }

        database.record_login_success(&record.user.id)?;
        self.start_session(record.user.clone(), record.session_version);
        Ok(record.user)
    }

    pub fn logout(&self) {
        self.clear_session();
    }

    pub fn create_user(
        &self,
        database: &Database,
        input: CreateUserInput,
        actor_id: &str,
    ) -> AppResult<UserView> {
        validate_username(&input.username)?;
        validate_display_name(&input.display_name)?;
        validate_password(&input.password, &input.username)?;
        validate_location_scope(input.role, &input.location_ids)?;
        let password = Zeroizing::new(input.password);
        let password_hash = hash_password(password.as_str())?;
        database.create_user(
            input.username.trim(),
            input.display_name.trim(),
            input.role,
            &input.location_ids,
            &password_hash,
            actor_id,
        )
    }

    pub fn update_user(
        &self,
        database: &Database,
        input: UpdateUserInput,
        actor_id: &str,
    ) -> AppResult<UserView> {
        validate_display_name(&input.display_name)?;
        validate_location_scope(input.role, &input.location_ids)?;
        database.update_user(input, actor_id)
    }

    pub fn reset_user_password(
        &self,
        database: &Database,
        input: ResetUserPasswordInput,
        actor_id: &str,
    ) -> AppResult<UserView> {
        let username = database.user_username(&input.id)?;
        validate_password(&input.password, &username)?;
        let password = Zeroizing::new(input.password);
        let password_hash = hash_password(password.as_str())?;
        database.reset_user_password(&input.id, &password_hash, actor_id)
    }

    pub fn require(
        &self,
        database: &Database,
        permission: Permission,
    ) -> AppResult<AuthenticatedUser> {
        let user = self.current_user(database)?;
        if !is_allowed(user.role, permission) {
            return Err(AppError::Forbidden(
                "Seu perfil não possui permissão para esta operação".into(),
            ));
        }
        Ok(user)
    }

    pub fn reauthenticate(
        &self,
        database: &Database,
        permission: Permission,
        password: String,
    ) -> AppResult<AuthenticatedUser> {
        let actor = self.require(database, permission)?;
        let password = Zeroizing::new(password);
        let record = database
            .find_user_for_login(&actor.username)?
            .ok_or_else(invalid_credentials)?;
        if record
            .locked_until
            .as_deref()
            .and_then(parse_timestamp)
            .is_some_and(|until| until > Utc::now())
        {
            self.clear_session();
            return Err(invalid_credentials());
        }
        if record.user.id != actor.id || !verify_password(password.as_str(), &record.password_hash)
        {
            database.record_sensitive_reauthentication_failure(
                &record.user.id,
                record.failed_login_attempts,
            )?;
            if record.failed_login_attempts >= 4 {
                self.clear_session();
            }
            return Err(invalid_credentials());
        }
        database.record_sensitive_reauthentication_success(&actor.id)?;
        Ok(actor)
    }

    fn current_user(&self, database: &Database) -> AppResult<AuthenticatedUser> {
        let snapshot = self.session.lock().clone().ok_or_else(session_required)?;
        let now = Utc::now();
        let idle = now
            .signed_duration_since(snapshot.last_activity_at)
            .to_std()
            .unwrap_or_default();
        let absolute = now
            .signed_duration_since(snapshot.created_at)
            .to_std()
            .unwrap_or_default();
        if idle >= IDLE_TIMEOUT || absolute >= ABSOLUTE_TIMEOUT {
            self.clear_session();
            return Err(AppError::Unauthorized(
                "A sessão expirou. Entre novamente".into(),
            ));
        }

        let current = database.current_session_user(&snapshot.user.id, snapshot.session_version)?;
        let Some(current) = current else {
            self.clear_session();
            return Err(session_required());
        };
        let mut guard = self.session.lock();
        if let Some(session) = guard.as_mut() {
            session.user = current.clone();
            session.last_activity_at = now;
        }
        Ok(current)
    }

    fn start_session(&self, user: AuthenticatedUser, session_version: i64) {
        let now = Utc::now();
        *self.session.lock() = Some(Session {
            user,
            session_version,
            created_at: now,
            last_activity_at: now,
        });
    }

    fn clear_session(&self) {
        *self.session.lock() = None;
    }
}

fn is_allowed(role: UserRole, permission: Permission) -> bool {
    match permission {
        Permission::ReadInventory
        | Permission::RequestAdjustment
        | Permission::RequestReplenishment
        | Permission::ReceiveReplenishmentTransfer
        | Permission::RecordMovement
        | Permission::UpdateOperationalAssetStatus
        | Permission::ReportAssetIncident => true,
        Permission::ManageInventory
        | Permission::ReviewAdjustment
        | Permission::ReviewReplenishment
        | Permission::DispatchReplenishmentTransfer
        | Permission::ReviewAssetIncident
        | Permission::ViewAudit
        | Permission::ManageOperations => matches!(role, UserRole::Manager | UserRole::Admin),
        Permission::RestoreBackup
        | Permission::ManageUsers
        | Permission::ManageEncryption
        | Permission::ManageLocations => role == UserRole::Admin,
    }
}

fn password_hasher() -> AppResult<Argon2<'static>> {
    let params = Params::new(19 * 1024, 2, 1, None)
        .map_err(|error| AppError::Security(error.to_string()))?;
    Ok(Argon2::new(Algorithm::Argon2id, Version::V0x13, params))
}

fn hash_password(password: &str) -> AppResult<String> {
    let salt = SaltString::generate(&mut OsRng);
    password_hasher()?
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|error| AppError::Security(format!("Não foi possível proteger a senha: {error}")))
}

fn verify_password(password: &str, encoded: &str) -> bool {
    PasswordHash::new(encoded).ok().is_some_and(|hash| {
        password_hasher()
            .is_ok_and(|argon2| argon2.verify_password(password.as_bytes(), &hash).is_ok())
    })
}

fn validate_username(username: &str) -> AppResult<()> {
    let username = username.trim();
    if !(3..=64).contains(&username.chars().count())
        || !username
            .chars()
            .all(|character| character.is_alphanumeric() || matches!(character, '.' | '_' | '-'))
    {
        return Err(AppError::Validation(
            "O usuário deve ter de 3 a 64 caracteres e usar apenas letras, números, ponto, hífen ou sublinhado".into(),
        ));
    }
    Ok(())
}

fn validate_display_name(display_name: &str) -> AppResult<()> {
    if !(3..=100).contains(&display_name.trim().chars().count()) {
        return Err(AppError::Validation(
            "O nome deve ter de 3 a 100 caracteres".into(),
        ));
    }
    Ok(())
}

fn validate_location_scope(role: UserRole, location_ids: &[String]) -> AppResult<()> {
    if role != UserRole::Admin && location_ids.is_empty() {
        return Err(AppError::Validation(
            "Operadores e gestores devem estar vinculados a pelo menos uma unidade".into(),
        ));
    }
    Ok(())
}

fn validate_password(password: &str, username: &str) -> AppResult<()> {
    let length = password.chars().count();
    if !(PASSWORD_MIN_CHARS..=PASSWORD_MAX_CHARS).contains(&length) {
        return Err(AppError::Validation(format!(
            "A senha deve ter entre {PASSWORD_MIN_CHARS} e {PASSWORD_MAX_CHARS} caracteres"
        )));
    }
    if password
        .to_lowercase()
        .contains(&username.trim().to_lowercase())
    {
        return Err(AppError::Validation(
            "A senha não deve conter o nome de usuário".into(),
        ));
    }
    Ok(())
}

fn parse_timestamp(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|value| value.with_timezone(&Utc))
}

fn invalid_credentials() -> AppError {
    AppError::Unauthorized("Usuário ou senha inválidos".into())
}

fn session_required() -> AppError {
    AppError::Unauthorized("Entre para continuar".into())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn hashes_and_verifies_password_with_argon2id() {
        let hash = hash_password("frase-segura-com-acentuação").expect("hash");
        assert!(hash.starts_with("$argon2id$"));
        assert!(verify_password("frase-segura-com-acentuação", &hash));
        assert!(!verify_password("senha-incorreta", &hash));
    }

    #[test]
    fn enforces_role_permissions() {
        assert!(is_allowed(UserRole::Operator, Permission::ReadInventory));
        assert!(!is_allowed(UserRole::Operator, Permission::ManageInventory));
        assert!(is_allowed(UserRole::Manager, Permission::ReviewAdjustment));
        assert!(!is_allowed(UserRole::Manager, Permission::ManageUsers));
        assert!(is_allowed(UserRole::Admin, Permission::RestoreBackup));
    }

    #[test]
    fn locks_and_clears_session_after_sensitive_reauthentication_failures() {
        let database = Database::open(PathBuf::from(":memory:")).expect("database");
        let auth = AuthService::new().expect("auth service");
        auth.bootstrap(
            &database,
            BootstrapAdminInput {
                username: "admin".into(),
                display_name: "Administrador Principal".into(),
                password: "frase-segura-exclusiva-2026".into(),
            },
        )
        .expect("bootstrap");
        for _ in 0..5 {
            assert!(auth
                .reauthenticate(
                    &database,
                    Permission::ManageEncryption,
                    "senha-incorreta".into(),
                )
                .is_err());
        }
        assert!(auth.status(&database).expect("status").user.is_none());
    }
}
