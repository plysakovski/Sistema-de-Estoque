use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

use chrono::{DateTime, Duration as ChronoDuration, NaiveDate, Utc};
use parking_lot::Mutex;
use rusqlite::{backup::Backup, params, Connection, OptionalExtension, Transaction};
use uuid::Uuid;

use crate::{
    encryption::{open_encrypted, open_encrypted_read_only, unlock, StorageKey},
    error::{AppError, AppResult},
    imports::{normalize_key, ImportContext, ImportResult, StagedImport},
    models::{
        AdjustmentRequestView, AssetFilterInput, AssetIncidentView, AssetRecordView, AssetView,
        AuditFilterInput, AuditLogView, AuthUserRecord, AuthenticatedUser, BackupRecord,
        CreateAdjustmentRequestInput, CreateAssetIncidentInput, CreateItemInput,
        CreateMovementBatchInput, CreateMovementInput, CreateReplenishmentRequestInput, Dashboard,
        DispatchReplenishmentTransferInput, InventoryItem, LocationOption, LocationTotal,
        MasterDataRecord, MovementBatchView, MovementView, ReceiveReplenishmentTransferInput,
        RecoveryKeyStatus, ReplenishmentRequestItemView, ReplenishmentRequestView,
        ReplenishmentTransferShipmentItemView, ReplenishmentTransferShipmentView,
        ReservedTransferAssetView, RestoreResult, ReviewAdjustmentRequestInput,
        ReviewAssetIncidentInput, ReviewReplenishmentRequestInput, SaveMasterDataInput,
        UpdateAssetInput, UpdateUserInput, UserRole, UserView,
    },
};

const MIGRATION_0001: &str = include_str!("../migrations/0001_initial.sql");
const MIGRATION_0002: &str = include_str!("../migrations/0002_hybrid_inventory.sql");
const MIGRATION_0003: &str =
    include_str!("../migrations/0003_security_and_adjustment_requests.sql");
const MIGRATION_0004: &str = include_str!("../migrations/0004_scopes_and_replenishment.sql");
const MIGRATION_0005: &str = include_str!("../migrations/0005_batches_and_asset_incidents.sql");
const MIGRATION_0006: &str = include_str!("../migrations/0006_two_step_transfers.sql");
const ADMIN_ID: &str = "123e4567-e89b-42d3-a456-426614174000";

#[derive(Clone, Copy)]
enum MasterDataKind {
    Location,
    Category,
}

pub struct Database {
    connection: Mutex<Connection>,
    path: PathBuf,
    storage_key: StorageKey,
}

impl Database {
    pub fn open(path: PathBuf) -> AppResult<Self> {
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            fs::create_dir_all(parent)?;
        }
        let storage_key = resolve_storage_key(&path)?;
        let connection = open_or_encrypt_database(&path, &storage_key)?;
        connection.execute_batch(
            "PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL; PRAGMA busy_timeout = 5000;",
        )?;
        connection.execute_batch(MIGRATION_0001)?;
        apply_migration(&connection, 2, MIGRATION_0002)?;
        apply_migration(&connection, 3, MIGRATION_0003)?;
        apply_migration(&connection, 4, MIGRATION_0004)?;
        apply_migration(&connection, 5, MIGRATION_0005)?;
        apply_migration(&connection, 6, MIGRATION_0006)?;
        if cfg!(debug_assertions) {
            seed_database(&connection)?;
        }
        Ok(Self {
            connection: Mutex::new(connection),
            path,
            storage_key,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn recovery_key_status(&self) -> AppResult<RecoveryKeyStatus> {
        let connection = self.connection.lock();
        let acknowledged = connection
            .query_row(
                "SELECT value FROM app_settings WHERE key = 'recovery_key_acknowledged'",
                [],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .is_some_and(|value| value == "true");
        Ok(RecoveryKeyStatus { acknowledged })
    }

    pub fn reveal_recovery_key(&self, actor_id: &str) -> AppResult<String> {
        let mut connection = self.connection.lock();
        let transaction = connection.transaction()?;
        let now = Utc::now().to_rfc3339();
        insert_audit(
            &transaction,
            "reveal_recovery_key",
            "security_configuration",
            "database_encryption",
            None,
            Some(serde_json::json!({"revealed": true})),
            actor_id,
            &now,
        )?;
        transaction.commit()?;
        Ok(self.storage_key.expose().to_owned())
    }

    pub fn acknowledge_recovery_key(&self, actor_id: &str) -> AppResult<RecoveryKeyStatus> {
        let mut connection = self.connection.lock();
        let transaction = connection.transaction()?;
        let now = Utc::now().to_rfc3339();
        transaction.execute(
            "INSERT INTO app_settings(key, value, updated_at) VALUES ('recovery_key_acknowledged', 'true', ?1) ON CONFLICT(key) DO UPDATE SET value = 'true', updated_at = excluded.updated_at",
            [&now],
        )?;
        insert_audit(
            &transaction,
            "acknowledge_recovery_key",
            "security_configuration",
            "database_encryption",
            None,
            Some(serde_json::json!({"storedOffline": true})),
            actor_id,
            &now,
        )?;
        transaction.commit()?;
        Ok(RecoveryKeyStatus { acknowledged: true })
    }

    pub fn auth_setup_required(&self) -> AppResult<bool> {
        let connection = self.connection.lock();
        let configured: bool = connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM users WHERE active = 1 AND password_hash IS NOT NULL)",
            [],
            |row| row.get(0),
        )?;
        Ok(!configured)
    }

    pub fn bootstrap_admin(
        &self,
        username: &str,
        display_name: &str,
        password_hash: &str,
    ) -> AppResult<(AuthenticatedUser, i64)> {
        let mut connection = self.connection.lock();
        let transaction = connection.transaction()?;
        let configured: bool = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM users WHERE password_hash IS NOT NULL)",
            [],
            |row| row.get(0),
        )?;
        if configured {
            return Err(AppError::Conflict(
                "A configuração inicial já foi concluída".into(),
            ));
        }
        let id = transaction
            .query_row(
                "SELECT id FROM users WHERE role = 'admin' ORDER BY created_at LIMIT 1",
                [],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        let now = Utc::now().to_rfc3339();
        transaction.execute(
            "INSERT INTO users(id, username, display_name, role, active, created_at, updated_at, password_hash, password_changed_at, session_version) VALUES (?1, ?2, ?3, 'admin', 1, ?4, ?4, ?5, ?4, 1) ON CONFLICT(id) DO UPDATE SET username = excluded.username, display_name = excluded.display_name, role = 'admin', active = 1, password_hash = excluded.password_hash, password_changed_at = excluded.password_changed_at, failed_login_attempts = 0, locked_until = NULL, session_version = users.session_version + 1, updated_at = excluded.updated_at",
            params![id, username, display_name, now, password_hash],
        )?;
        let session_version: i64 = transaction.query_row(
            "SELECT session_version FROM users WHERE id = ?1",
            [&id],
            |row| row.get(0),
        )?;
        transaction.execute(
            "INSERT INTO audit_logs(id, actor_id, action, entity_type, entity_id, before_data, after_data, occurred_at) VALUES (?1, ?2, 'bootstrap_admin', 'user', ?2, NULL, ?3, ?4)",
            params![Uuid::new_v4().to_string(), id, serde_json::json!({"username": username, "displayName": display_name, "role": "admin"}).to_string(), now],
        )?;
        transaction.commit()?;
        Ok((
            AuthenticatedUser {
                id,
                username: username.to_owned(),
                display_name: display_name.to_owned(),
                role: UserRole::Admin,
            },
            session_version,
        ))
    }

    pub(crate) fn find_user_for_login(&self, username: &str) -> AppResult<Option<AuthUserRecord>> {
        let connection = self.connection.lock();
        let record = connection
            .query_row(
                "SELECT id, username, display_name, role, password_hash, failed_login_attempts, locked_until, session_version FROM users WHERE username = ?1 COLLATE NOCASE AND active = 1 AND password_hash IS NOT NULL",
                [username],
                map_auth_user,
            )
            .optional()?;
        Ok(record)
    }

    pub(crate) fn current_session_user(
        &self,
        user_id: &str,
        session_version: i64,
    ) -> AppResult<Option<AuthenticatedUser>> {
        let connection = self.connection.lock();
        let user = connection
            .query_row(
                "SELECT id, username, display_name, role FROM users WHERE id = ?1 AND active = 1 AND session_version = ?2 AND password_hash IS NOT NULL",
                params![user_id, session_version],
                map_authenticated_user,
            )
            .optional()?;
        Ok(user)
    }

    pub(crate) fn record_login_failure(
        &self,
        user_id: &str,
        previous_failures: i64,
    ) -> AppResult<()> {
        self.record_authentication_failure(user_id, previous_failures, "login_failed")
    }

    pub(crate) fn record_sensitive_reauthentication_failure(
        &self,
        user_id: &str,
        previous_failures: i64,
    ) -> AppResult<()> {
        self.record_authentication_failure(
            user_id,
            previous_failures,
            "sensitive_reauthentication_failed",
        )
    }

    fn record_authentication_failure(
        &self,
        user_id: &str,
        previous_failures: i64,
        action: &str,
    ) -> AppResult<()> {
        let mut connection = self.connection.lock();
        let transaction = connection.transaction()?;
        let failures = previous_failures.saturating_add(1);
        let now = Utc::now();
        let locked_until =
            (failures >= 5).then(|| (now + ChronoDuration::minutes(15)).to_rfc3339());
        transaction.execute(
            "UPDATE users SET failed_login_attempts = ?2, locked_until = ?3, updated_at = ?4 WHERE id = ?1",
            params![user_id, failures, locked_until, now.to_rfc3339()],
        )?;
        transaction.execute(
            "INSERT INTO audit_logs(id, actor_id, action, entity_type, entity_id, before_data, after_data, occurred_at) VALUES (?1, ?2, ?3, 'authentication', ?2, NULL, ?4, ?5)",
            params![Uuid::new_v4().to_string(), user_id, action, serde_json::json!({"failedAttempts": failures, "temporarilyLocked": locked_until.is_some()}).to_string(), now.to_rfc3339()],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub(crate) fn record_login_success(&self, user_id: &str) -> AppResult<()> {
        let mut connection = self.connection.lock();
        let transaction = connection.transaction()?;
        let now = Utc::now().to_rfc3339();
        transaction.execute(
            "UPDATE users SET failed_login_attempts = 0, locked_until = NULL, last_login_at = ?2, updated_at = ?2 WHERE id = ?1",
            params![user_id, now],
        )?;
        transaction.execute(
            "INSERT INTO audit_logs(id, actor_id, action, entity_type, entity_id, before_data, after_data, occurred_at) VALUES (?1, ?2, 'login_success', 'authentication', ?2, NULL, NULL, ?3)",
            params![Uuid::new_v4().to_string(), user_id, now],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub(crate) fn record_sensitive_reauthentication_success(&self, user_id: &str) -> AppResult<()> {
        let mut connection = self.connection.lock();
        let transaction = connection.transaction()?;
        let now = Utc::now().to_rfc3339();
        transaction.execute(
            "UPDATE users SET failed_login_attempts = 0, locked_until = NULL, updated_at = ?2 WHERE id = ?1",
            params![user_id, now],
        )?;
        transaction.execute(
            "INSERT INTO audit_logs(id, actor_id, action, entity_type, entity_id, before_data, after_data, occurred_at) VALUES (?1, ?2, 'sensitive_reauthentication_success', 'authentication', ?2, NULL, NULL, ?3)",
            params![Uuid::new_v4().to_string(), user_id, now],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn list_users(&self) -> AppResult<Vec<UserView>> {
        let connection = self.connection.lock();
        let mut statement = connection.prepare(
            "SELECT id, username, display_name, role, active, failed_login_attempts, locked_until, last_login_at, created_at, updated_at FROM users WHERE password_hash IS NOT NULL ORDER BY active DESC, display_name COLLATE NOCASE",
        )?;
        let mut users = statement
            .query_map([], map_user_view)?
            .collect::<Result<Vec<_>, _>>()?;
        drop(statement);
        for user in &mut users {
            user.location_ids = query_user_location_ids(&connection, &user.id)?;
        }
        Ok(users)
    }

    pub fn user_username(&self, id: &str) -> AppResult<String> {
        let connection = self.connection.lock();
        connection
            .query_row(
                "SELECT username FROM users WHERE id = ?1 AND password_hash IS NOT NULL",
                [id],
                |row| row.get(0),
            )
            .optional()?
            .ok_or_else(|| AppError::NotFound(id.to_owned()))
    }

    pub fn create_user(
        &self,
        username: &str,
        display_name: &str,
        role: UserRole,
        location_ids: &[String],
        password_hash: &str,
        actor_id: &str,
    ) -> AppResult<UserView> {
        let mut connection = self.connection.lock();
        let transaction = connection.transaction()?;
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        transaction.execute(
            "INSERT INTO users(id, username, display_name, role, active, created_at, updated_at, password_hash, password_changed_at) VALUES (?1, ?2, ?3, ?4, 1, ?5, ?5, ?6, ?5)",
            params![id, username, display_name, role.as_str(), now, password_hash],
        )?;
        sync_user_locations(&transaction, &id, role, location_ids, &now)?;
        insert_audit(
            &transaction,
            "create",
            "user",
            &id,
            None,
            Some(
                serde_json::json!({"username": username, "displayName": display_name, "role": role.as_str(), "active": true, "locationIds": location_ids}),
            ),
            actor_id,
            &now,
        )?;
        transaction.commit()?;
        drop(connection);
        self.get_user(&id)
    }

    pub fn update_user(&self, input: UpdateUserInput, actor_id: &str) -> AppResult<UserView> {
        let mut connection = self.connection.lock();
        let transaction = connection.transaction()?;
        let current: (String, String, bool) = transaction
            .query_row(
                "SELECT display_name, role, active FROM users WHERE id = ?1 AND password_hash IS NOT NULL",
                [&input.id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?
            .ok_or_else(|| AppError::NotFound(input.id.clone()))?;
        if input.id == actor_id && (input.role != UserRole::Admin || !input.active) {
            return Err(AppError::Forbidden(
                "Você não pode remover o próprio acesso administrativo".into(),
            ));
        }
        if current.1 == "admin" && current.2 && (input.role != UserRole::Admin || !input.active) {
            let active_admins: i64 = transaction.query_row(
                "SELECT COUNT(*) FROM users WHERE role = 'admin' AND active = 1 AND password_hash IS NOT NULL",
                [],
                |row| row.get(0),
            )?;
            if active_admins <= 1 {
                return Err(AppError::Conflict(
                    "O último administrador ativo não pode ser removido ou rebaixado".into(),
                ));
            }
        }
        let now = Utc::now().to_rfc3339();
        transaction.execute(
            "UPDATE users SET display_name = ?2, role = ?3, active = ?4, session_version = session_version + 1, updated_at = ?5 WHERE id = ?1",
            params![input.id, input.display_name.trim(), input.role.as_str(), input.active, now],
        )?;
        sync_user_locations(
            &transaction,
            &input.id,
            input.role,
            &input.location_ids,
            &now,
        )?;
        insert_audit(
            &transaction,
            "update",
            "user",
            &input.id,
            Some(
                serde_json::json!({"displayName": current.0, "role": current.1, "active": current.2}),
            ),
            Some(
                serde_json::json!({"displayName": input.display_name.trim(), "role": input.role.as_str(), "active": input.active, "locationIds": input.location_ids}),
            ),
            actor_id,
            &now,
        )?;
        transaction.commit()?;
        drop(connection);
        self.get_user(&input.id)
    }

    pub fn reset_user_password(
        &self,
        id: &str,
        password_hash: &str,
        actor_id: &str,
    ) -> AppResult<UserView> {
        let mut connection = self.connection.lock();
        let transaction = connection.transaction()?;
        let now = Utc::now().to_rfc3339();
        let updated = transaction.execute(
            "UPDATE users SET password_hash = ?2, password_changed_at = ?3, failed_login_attempts = 0, locked_until = NULL, session_version = session_version + 1, updated_at = ?3 WHERE id = ?1 AND password_hash IS NOT NULL",
            params![id, password_hash, now],
        )?;
        if updated != 1 {
            return Err(AppError::NotFound(id.to_owned()));
        }
        insert_audit(
            &transaction,
            "reset_password",
            "user",
            id,
            None,
            Some(serde_json::json!({"sessionsInvalidated": true})),
            actor_id,
            &now,
        )?;
        transaction.commit()?;
        drop(connection);
        self.get_user(id)
    }

    fn get_user(&self, id: &str) -> AppResult<UserView> {
        let connection = self.connection.lock();
        let mut user = connection
            .query_row(
                "SELECT id, username, display_name, role, active, failed_login_attempts, locked_until, last_login_at, created_at, updated_at FROM users WHERE id = ?1 AND password_hash IS NOT NULL",
                [id],
                map_user_view,
            )
            .optional()?
            .ok_or_else(|| AppError::NotFound(id.to_owned()))?;
        user.location_ids = query_user_location_ids(&connection, id)?;
        Ok(user)
    }

    #[allow(dead_code)]
    pub fn list_items(&self, search: &str) -> AppResult<Vec<InventoryItem>> {
        let connection = self.connection.lock();
        query_items(&connection, search, false)
    }

    pub fn list_items_for(
        &self,
        search: &str,
        actor: &AuthenticatedUser,
    ) -> AppResult<Vec<InventoryItem>> {
        let connection = self.connection.lock();
        query_items_for(&connection, search, false, actor)
    }

    pub fn list_locations(&self) -> AppResult<Vec<LocationOption>> {
        let connection = self.connection.lock();
        let mut statement = connection.prepare(
            "SELECT id, name, code FROM locations WHERE active = 1 ORDER BY name COLLATE NOCASE",
        )?;
        let locations = statement
            .query_map([], |row| {
                Ok(LocationOption {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    code: row.get(2)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(locations)
    }

    pub fn list_locations_for(&self, actor: &AuthenticatedUser) -> AppResult<Vec<LocationOption>> {
        if actor.role == UserRole::Admin {
            return self.list_locations();
        }
        let connection = self.connection.lock();
        let mut statement = connection.prepare(
            "SELECT l.id, l.name, l.code
             FROM locations l
             JOIN user_locations ul ON ul.location_id = l.id
             WHERE ul.user_id = ?1 AND l.active = 1
             ORDER BY l.name COLLATE NOCASE",
        )?;
        let locations = statement
            .query_map([&actor.id], |row| {
                Ok(LocationOption {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    code: row.get(2)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(locations)
    }

    pub fn list_location_records(&self) -> AppResult<Vec<MasterDataRecord>> {
        let connection = self.connection.lock();
        query_master_data(&connection, MasterDataKind::Location)
    }

    pub fn save_location(
        &self,
        input: SaveMasterDataInput,
        actor_id: &str,
    ) -> AppResult<MasterDataRecord> {
        self.save_master_data(MasterDataKind::Location, input, actor_id)
    }

    pub fn set_location_active(
        &self,
        id: &str,
        active: bool,
        actor_id: &str,
    ) -> AppResult<MasterDataRecord> {
        self.set_master_data_active(MasterDataKind::Location, id, active, actor_id)
    }

    pub fn list_categories(&self) -> AppResult<Vec<MasterDataRecord>> {
        let connection = self.connection.lock();
        query_master_data(&connection, MasterDataKind::Category)
    }

    pub fn save_category(
        &self,
        input: SaveMasterDataInput,
        actor_id: &str,
    ) -> AppResult<MasterDataRecord> {
        self.save_master_data(MasterDataKind::Category, input, actor_id)
    }

    pub fn set_category_active(
        &self,
        id: &str,
        active: bool,
        actor_id: &str,
    ) -> AppResult<MasterDataRecord> {
        self.set_master_data_active(MasterDataKind::Category, id, active, actor_id)
    }

    pub fn list_assets(&self, product_id: &str) -> AppResult<Vec<AssetView>> {
        let connection = self.connection.lock();
        let mut statement = connection.prepare(
            "SELECT a.id, a.product_id, a.asset_tag, a.serial_number, a.location_id, l.name, a.status, a.receipt_batch_id, a.updated_at
             FROM assets a JOIN locations l ON l.id = a.location_id
             WHERE a.product_id = ?1 AND a.status <> 'disposed'
             ORDER BY a.asset_tag COLLATE NOCASE",
        )?;
        let assets = statement
            .query_map([product_id], map_asset)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(assets)
    }

    pub fn list_assets_for(
        &self,
        product_id: &str,
        actor: &AuthenticatedUser,
    ) -> AppResult<Vec<AssetView>> {
        let assets = self.list_assets(product_id)?;
        if actor.role == UserRole::Admin {
            return Ok(assets);
        }
        let allowed = self.user_location_set(&actor.id)?;
        Ok(assets
            .into_iter()
            .filter(|asset| allowed.contains(&asset.location_id))
            .collect())
    }

    pub fn list_asset_records(&self, filters: AssetFilterInput) -> AppResult<Vec<AssetRecordView>> {
        validate_asset_filters(&filters)?;
        let connection = self.connection.lock();
        query_asset_records(&connection, &filters)
    }

    pub fn list_asset_records_for(
        &self,
        filters: AssetFilterInput,
        actor: &AuthenticatedUser,
    ) -> AppResult<Vec<AssetRecordView>> {
        let records = self.list_asset_records(filters)?;
        if actor.role == UserRole::Admin {
            return Ok(records);
        }
        let allowed = self.user_location_set(&actor.id)?;
        Ok(records
            .into_iter()
            .filter(|asset| allowed.contains(&asset.location_id))
            .collect())
    }

    fn user_location_set(&self, user_id: &str) -> AppResult<HashSet<String>> {
        let connection = self.connection.lock();
        Ok(query_user_location_ids(&connection, user_id)?
            .into_iter()
            .collect())
    }

    pub fn update_asset(
        &self,
        input: UpdateAssetInput,
        actor_id: &str,
    ) -> AppResult<AssetRecordView> {
        validate_asset_update(&input)?;
        let mut connection = self.connection.lock();
        let transaction = connection.transaction()?;
        let current: (String, String, String, Option<String>) = transaction
            .query_row(
                "SELECT product_id, location_id, status, in_transit_shipment_id FROM assets WHERE id = ?1",
                [&input.id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .optional()?
            .ok_or_else(|| AppError::NotFound(input.id.clone()))?;

        if current.2 == "disposed" {
            return Err(AppError::Validation(
                "Um ativo baixado não pode ser alterado".into(),
            ));
        }
        if current.3.is_some() {
            return Err(AppError::Conflict(
                "O ativo está em trânsito e só pode ser alterado pelo recebimento da transferência"
                    .into(),
            ));
        }
        ensure_location(&transaction, &input.location_id)?;
        let location_changed = current.1 != input.location_id;
        let status_changed = current.2 != input.status;
        if !location_changed && !status_changed {
            return Err(AppError::Validation(
                "Nenhuma alteração foi informada".into(),
            ));
        }

        let now = Utc::now().to_rfc3339();
        transaction.execute(
            "UPDATE assets SET location_id = ?2, status = ?3, version = version + 1, updated_at = ?4 WHERE id = ?1",
            params![input.id, input.location_id, input.status, now],
        )?;
        if location_changed {
            insert_movement(
                &transaction,
                &current.0,
                Some(&input.id),
                "transfer",
                1,
                Some(&current.1),
                Some(&input.location_id),
                input.note.trim(),
                actor_id,
                &now,
            )?;
        }
        if status_changed {
            insert_movement(
                &transaction,
                &current.0,
                Some(&input.id),
                "adjustment",
                1,
                Some(&input.location_id),
                None,
                input.note.trim(),
                actor_id,
                &now,
            )?;
        }
        transaction.execute(
            "UPDATE products SET version = version + 1, updated_at = ?2 WHERE id = ?1",
            params![current.0, now],
        )?;
        insert_audit(
            &transaction,
            "update",
            "asset",
            &input.id,
            Some(serde_json::json!({"locationId": current.1, "status": current.2})),
            Some(
                serde_json::json!({"locationId": input.location_id, "status": input.status, "note": input.note.trim()}),
            ),
            actor_id,
            &now,
        )?;
        transaction.commit()?;
        drop(connection);
        self.get_asset_record(&input.id)
    }

    pub fn update_asset_for(
        &self,
        input: UpdateAssetInput,
        actor: &AuthenticatedUser,
    ) -> AppResult<AssetRecordView> {
        if actor.role != UserRole::Admin {
            let connection = self.connection.lock();
            let (current_location, current_status): (String, String) = connection
                .query_row(
                    "SELECT location_id, status FROM assets WHERE id = ?1",
                    [&input.id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()?
                .ok_or_else(|| AppError::NotFound(input.id.clone()))?;
            ensure_actor_location_connection(&connection, actor, &current_location)?;
            ensure_actor_location_connection(&connection, actor, &input.location_id)?;
            if actor.role == UserRole::Operator {
                if input.location_id != current_location {
                    return Err(AppError::Forbidden(
                        "Operadores não podem transferir ativos entre unidades".into(),
                    ));
                }
                let operational_transition = matches!(
                    (current_status.as_str(), input.status.as_str()),
                    ("available", "in_use") | ("in_use", "available")
                );
                if !operational_transition {
                    return Err(AppError::Forbidden(
                        "Operadores podem alternar apenas entre Disponível e Em uso; manutenção ou correção exige gestor ou solicitação de ajuste".into(),
                    ));
                }
            }
        }
        self.update_asset(input, &actor.id)
    }

    pub fn list_audit_logs(&self, filters: AuditFilterInput) -> AppResult<Vec<AuditLogView>> {
        validate_audit_filters(&filters)?;
        let connection = self.connection.lock();
        query_audit_logs(&connection, &filters)
    }

    pub fn list_movements(&self) -> AppResult<Vec<MovementView>> {
        let connection = self.connection.lock();
        query_movements(&connection, 200)
    }

    pub fn list_movements_for(&self, actor: &AuthenticatedUser) -> AppResult<Vec<MovementView>> {
        if actor.role == UserRole::Admin {
            return self.list_movements();
        }
        let connection = self.connection.lock();
        let sql = format!(
            "{MOVEMENT_SELECT}
             WHERE EXISTS(
                 SELECT 1 FROM user_locations ul
                 WHERE ul.user_id = ?1
                   AND (ul.location_id = m.from_location_id OR ul.location_id = m.to_location_id)
             )
             ORDER BY m.occurred_at DESC, m.rowid DESC LIMIT 200"
        );
        let mut statement = connection.prepare(&sql)?;
        let movements = statement
            .query_map([&actor.id], map_movement)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(movements)
    }

    pub fn dashboard(&self) -> AppResult<Dashboard> {
        let connection = self.connection.lock();
        let total_items = connection.query_row(
            "SELECT COALESCE((SELECT SUM(quantity) FROM stock_balances), 0) + COALESCE((SELECT COUNT(*) FROM assets WHERE status <> 'disposed'), 0)",
            [],
            |row| row.get(0),
        )?;
        let location_count = connection.query_row(
            "SELECT COUNT(*) FROM locations WHERE active = 1",
            [],
            |row| row.get(0),
        )?;
        let category_count = connection.query_row(
            "SELECT COUNT(*) FROM categories WHERE active = 1",
            [],
            |row| row.get(0),
        )?;
        let attention_count = connection.query_row(
            "SELECT COUNT(*) FROM products p WHERE p.active = 1 AND p.tracking_type = 'quantity' AND COALESCE((SELECT SUM(sb.quantity) FROM stock_balances sb WHERE sb.product_id = p.id), 0) < p.minimum_quantity",
            [],
            |row| row.get(0),
        )?;
        let maintenance_count = connection.query_row(
            "SELECT COUNT(*) FROM assets WHERE status = 'maintenance'",
            [],
            |row| row.get(0),
        )?;
        let pending_adjustment_count = connection.query_row(
            "SELECT COUNT(*) FROM adjustment_requests WHERE status = 'pending'",
            [],
            |row| row.get(0),
        )?;

        let mut location_statement = connection.prepare(
            "SELECT l.name, COALESCE((SELECT SUM(sb.quantity) FROM stock_balances sb WHERE sb.location_id = l.id), 0) + COALESCE((SELECT COUNT(*) FROM assets a WHERE a.location_id = l.id AND a.status <> 'disposed'), 0) FROM locations l WHERE l.active = 1 ORDER BY 2 DESC, l.name",
        )?;
        let inventory_by_location = location_statement
            .query_map([], |row| {
                Ok(LocationTotal {
                    location: row.get(0)?,
                    quantity: row.get(1)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        drop(location_statement);

        Ok(Dashboard {
            total_items,
            location_count,
            category_count,
            attention_count,
            maintenance_count,
            pending_adjustment_count,
            inventory_by_location,
            low_stock: query_items(&connection, "", true)?,
            recent_movements: query_movements(&connection, 10)?,
            updated_at: Utc::now().to_rfc3339(),
        })
    }

    pub fn dashboard_for(&self, actor: &AuthenticatedUser) -> AppResult<Dashboard> {
        if actor.role == UserRole::Admin {
            return self.dashboard();
        }
        let items = self.list_items_for("", actor)?;
        let low_stock = {
            let connection = self.connection.lock();
            query_items_for(&connection, "", true, actor)?
        };
        let assets = self.list_asset_records_for(
            AssetFilterInput {
                search: String::new(),
                location_id: None,
                status: None,
            },
            actor,
        )?;
        let movements = self.list_movements_for(actor)?;
        let connection = self.connection.lock();
        let category_count = connection.query_row(
            "SELECT COUNT(*) FROM categories WHERE active = 1",
            [],
            |row| row.get(0),
        )?;
        let pending_adjustment_count = connection.query_row(
            "SELECT COUNT(*)
             FROM adjustment_requests ar
             LEFT JOIN assets a ON a.id = ar.asset_id
             WHERE ar.status = 'pending' AND EXISTS(
                 SELECT 1 FROM user_locations ul
                 WHERE ul.user_id = ?1 AND ul.location_id = COALESCE(ar.location_id, a.location_id)
             )",
            [&actor.id],
            |row| row.get(0),
        )?;
        let mut statement = connection.prepare(
            "SELECT l.name,
                COALESCE((SELECT SUM(sb.quantity) FROM stock_balances sb WHERE sb.location_id = l.id), 0)
                + COALESCE((SELECT COUNT(*) FROM assets a WHERE a.location_id = l.id AND a.status <> 'disposed'), 0)
             FROM locations l
             JOIN user_locations ul ON ul.location_id = l.id
             WHERE ul.user_id = ?1 AND l.active = 1
             ORDER BY 2 DESC, l.name",
        )?;
        let inventory_by_location = statement
            .query_map([&actor.id], |row| {
                Ok(LocationTotal {
                    location: row.get(0)?,
                    quantity: row.get(1)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Dashboard {
            total_items: items.iter().map(|item| item.quantity).sum(),
            location_count: inventory_by_location.len() as i64,
            category_count,
            attention_count: low_stock.len() as i64,
            maintenance_count: assets
                .iter()
                .filter(|asset| asset.status == "maintenance")
                .count() as i64,
            pending_adjustment_count,
            inventory_by_location,
            low_stock,
            recent_movements: movements.into_iter().take(10).collect(),
            updated_at: Utc::now().to_rfc3339(),
        })
    }

    pub fn create_item(&self, input: CreateItemInput, actor_id: &str) -> AppResult<InventoryItem> {
        validate_item(&input)?;
        let mut connection = self.connection.lock();
        let transaction = connection.transaction()?;
        let category_id = find_or_create_category(&transaction, &input.category)?;
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        transaction.execute(
            "INSERT INTO products(id, sku, name, category_id, tracking_type, minimum_quantity, serial_number_policy, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)",
            params![id, input.sku.trim(), input.name.trim(), category_id, input.tracking_type, input.minimum_quantity, input.serial_number_policy, now],
        )?;

        if input.initial_quantity > 0 {
            let location_id = required_value(&input.location_id, "Informe a unidade inicial")?;
            ensure_location(&transaction, location_id)?;
            let asset_id = if input.tracking_type == "quantity" {
                set_balance(&transaction, &id, location_id, input.initial_quantity, &now)?;
                None
            } else {
                let asset_tag = required_text(&input.asset_tag, "Informe o patrimônio do ativo")?;
                let asset_id = Uuid::new_v4().to_string();
                transaction.execute(
                    "INSERT INTO assets(id, product_id, asset_tag, serial_number, location_id, status, created_at, updated_at) VALUES (?1, ?2, ?3, NULLIF(?4, ''), ?5, 'available', ?6, ?6)",
                    params![asset_id, id, asset_tag, input.serial_number.as_deref().unwrap_or("").trim(), location_id, now],
                )?;
                Some(asset_id)
            };
            insert_movement(
                &transaction,
                &id,
                asset_id.as_deref(),
                "entry",
                input.initial_quantity,
                None,
                Some(location_id),
                "Cadastro inicial",
                actor_id,
                &now,
            )?;
        }

        insert_audit(
            &transaction,
            "create",
            "product",
            &id,
            None,
            Some(serde_json::json!({
                "sku": input.sku.trim(),
                "name": input.name.trim(),
                "trackingType": input.tracking_type,
                "initialQuantity": input.initial_quantity
            })),
            actor_id,
            &now,
        )?;
        transaction.commit()?;
        drop(connection);
        self.get_item(&id)
    }

    pub fn create_item_for(
        &self,
        input: CreateItemInput,
        actor: &AuthenticatedUser,
    ) -> AppResult<InventoryItem> {
        if let Some(location_id) = input.location_id.as_deref() {
            let connection = self.connection.lock();
            ensure_actor_location_connection(&connection, actor, location_id)?;
        }
        self.create_item(input, &actor.id)
    }

    pub fn create_movement(
        &self,
        input: CreateMovementInput,
        actor_id: &str,
    ) -> AppResult<MovementView> {
        validate_movement(&input)?;
        let mut connection = self.connection.lock();
        let transaction = connection.transaction()?;
        let tracking_type: String = transaction
            .query_row(
                "SELECT tracking_type FROM products WHERE id = ?1 AND active = 1",
                [&input.product_id],
                |row| row.get(0),
            )
            .optional()?
            .ok_or_else(|| AppError::NotFound(input.product_id.clone()))?;
        let now = Utc::now().to_rfc3339();
        let movement_id = Uuid::new_v4().to_string();

        let (asset_id, from_location_id, to_location_id, recorded_quantity, before, after) =
            if tracking_type == "quantity" {
                apply_quantity_movement(&transaction, &input, &now)?
            } else {
                apply_asset_movement(&transaction, &input, &now)?
            };

        transaction.execute(
            "INSERT INTO stock_movements(id, product_id, asset_id, kind, quantity, from_location_id, to_location_id, actor_id, note, occurred_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![movement_id, input.product_id, asset_id, input.kind, recorded_quantity, from_location_id, to_location_id, actor_id, input.note.trim(), now],
        )?;
        transaction.execute(
            "UPDATE products SET version = version + 1, updated_at = ?2 WHERE id = ?1",
            params![input.product_id, now],
        )?;
        insert_audit(
            &transaction,
            &input.kind,
            if tracking_type == "quantity" {
                "stock"
            } else {
                "asset"
            },
            asset_id.as_deref().unwrap_or(&input.product_id),
            Some(before),
            Some(after),
            actor_id,
            &now,
        )?;
        transaction.commit()?;
        drop(connection);
        self.get_movement(&movement_id)
    }

    pub fn create_scoped_movement(
        &self,
        input: CreateMovementInput,
        actor: &AuthenticatedUser,
    ) -> AppResult<MovementView> {
        if actor.role == UserRole::Operator && input.kind != "exit" {
            return Err(AppError::Forbidden(
                "Operadores podem registrar apenas saídas rotineiras; outras alterações exigem solicitação".into(),
            ));
        }
        if actor.role != UserRole::Admin {
            let connection = self.connection.lock();
            let mut location_ids = Vec::new();
            if let Some(location) = input.from_location_id.as_deref() {
                location_ids.push(location.to_owned());
            }
            if let Some(location) = input.to_location_id.as_deref() {
                location_ids.push(location.to_owned());
            }
            if let Some(asset_id) = input.asset_id.as_deref() {
                if let Some(location) = connection
                    .query_row(
                        "SELECT location_id FROM assets WHERE id = ?1",
                        [asset_id],
                        |row| row.get::<_, String>(0),
                    )
                    .optional()?
                {
                    location_ids.push(location);
                }
            }
            for location_id in location_ids {
                ensure_actor_location_connection(&connection, actor, &location_id)?;
            }
        }
        self.create_movement(input, &actor.id)
    }

    pub fn create_movement_batch(
        &self,
        input: CreateMovementBatchInput,
        actor: &AuthenticatedUser,
    ) -> AppResult<MovementBatchView> {
        if input.items.is_empty() || input.items.len() > 200 {
            return Err(AppError::Validation(
                "O lote deve conter entre 1 e 200 movimentações".into(),
            ));
        }
        if input.reference.chars().count() > 120 || input.note.chars().count() > 1000 {
            return Err(AppError::Validation(
                "Referência ou observação muito longa".into(),
            ));
        }
        if input.items.iter().any(|item| item.kind != input.kind) {
            return Err(AppError::Validation(
                "Todas as linhas do lote devem possuir o mesmo tipo".into(),
            ));
        }
        if actor.role == UserRole::Operator && input.kind != "exit" {
            return Err(AppError::Forbidden(
                "Operadores podem registrar em lote somente saídas rotineiras".into(),
            ));
        }

        let mut connection = self.connection.lock();
        let transaction = connection.transaction()?;
        let now = Utc::now().to_rfc3339();
        let batch_id = Uuid::new_v4().to_string();

        let replenishment_destination = if let Some(request_id) =
            input.replenishment_request_id.as_deref()
        {
            if actor.role == UserRole::Operator || input.kind != "entry" {
                return Err(AppError::Forbidden(
                    "O recebimento de reposição exige perfil gestor e lote de entrada".into(),
                ));
            }
            if input.reference.trim().is_empty() {
                return Err(AppError::Validation(
                    "Informe a nota fiscal ou referência do lote recebido".into(),
                ));
            }
            let destination = transaction
                .query_row(
                    "SELECT destination_location_id FROM replenishment_requests WHERE id = ?1 AND status IN ('approved', 'partially_approved', 'in_fulfillment')",
                    [request_id],
                    |row| row.get::<_, String>(0),
                )
                .optional()?
                .ok_or_else(|| AppError::Conflict("A reposição não está disponível para recebimento".into()))?;
            ensure_actor_location(&transaction, actor, &destination)?;
            Some(destination)
        } else {
            None
        };

        transaction.execute(
            "INSERT INTO movement_batches(id, kind, reference, note, replenishment_request_id, actor_id, occurred_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![batch_id, input.kind, input.reference.trim(), input.note.trim(), input.replenishment_request_id, actor.id, now],
        )?;

        let mut movement_ids = Vec::with_capacity(input.items.len());
        let mut received_by_product: HashMap<String, i64> = HashMap::new();
        for item in &input.items {
            validate_movement(item)?;
            for location_id in [
                item.from_location_id.as_deref(),
                item.to_location_id.as_deref(),
            ]
            .into_iter()
            .flatten()
            {
                ensure_actor_location(&transaction, actor, location_id)?;
            }
            if let Some(asset_id) = item.asset_id.as_deref() {
                let asset_location = transaction
                    .query_row(
                        "SELECT location_id FROM assets WHERE id = ?1",
                        [asset_id],
                        |row| row.get::<_, String>(0),
                    )
                    .optional()?
                    .ok_or_else(|| AppError::NotFound(asset_id.to_owned()))?;
                ensure_actor_location(&transaction, actor, &asset_location)?;
                if item
                    .from_location_id
                    .as_deref()
                    .is_some_and(|value| value != asset_location)
                {
                    return Err(AppError::Validation(
                        "A unidade de origem não corresponde ao ativo selecionado".into(),
                    ));
                }
            }
            if let Some(destination) = replenishment_destination.as_deref() {
                if item.to_location_id.as_deref() != Some(destination) {
                    return Err(AppError::Validation(
                        "Todas as entradas da reposição devem usar sua unidade de destino".into(),
                    ));
                }
            }

            let tracking_type: String = transaction
                .query_row(
                    "SELECT tracking_type FROM products WHERE id = ?1 AND active = 1",
                    [&item.product_id],
                    |row| row.get(0),
                )
                .optional()?
                .ok_or_else(|| AppError::NotFound(item.product_id.clone()))?;
            if tracking_type == "serialized" && item.kind == "entry" {
                let serial_policy: String = transaction.query_row(
                    "SELECT serial_number_policy FROM products WHERE id = ?1",
                    [&item.product_id],
                    |row| row.get(0),
                )?;
                let serial_number_missing = match item.serial_number.as_deref() {
                    Some(value) => value.trim().is_empty(),
                    None => true,
                };
                if serial_policy == "required" && serial_number_missing {
                    return Err(AppError::Validation(
                        "Informe o número de série de cada ativo deste produto".into(),
                    ));
                }
            }

            let movement_id = Uuid::new_v4().to_string();
            let (asset_id, from_location_id, to_location_id, recorded_quantity, before, after) =
                if tracking_type == "quantity" {
                    apply_quantity_movement(&transaction, item, &now)?
                } else {
                    apply_asset_movement(&transaction, item, &now)?
                };
            if tracking_type == "serialized" && item.kind == "entry" {
                if let Some(asset_id) = asset_id.as_deref() {
                    transaction.execute(
                        "UPDATE assets SET receipt_batch_id = ?2 WHERE id = ?1",
                        params![asset_id, batch_id],
                    )?;
                }
            }
            transaction.execute(
                "INSERT INTO stock_movements(id, product_id, asset_id, kind, quantity, from_location_id, to_location_id, actor_id, note, occurred_at, batch_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![movement_id, item.product_id, asset_id, item.kind, recorded_quantity, from_location_id, to_location_id, actor.id, item.note.trim(), now, batch_id],
            )?;
            transaction.execute(
                "UPDATE products SET version = version + 1, updated_at = ?2 WHERE id = ?1",
                params![item.product_id, now],
            )?;
            insert_audit(
                &transaction,
                &item.kind,
                if tracking_type == "quantity" {
                    "stock"
                } else {
                    "asset"
                },
                asset_id.as_deref().unwrap_or(&item.product_id),
                Some(before),
                Some(after),
                &actor.id,
                &now,
            )?;
            *received_by_product
                .entry(item.product_id.clone())
                .or_default() += recorded_quantity;
            movement_ids.push(movement_id);
        }

        if let Some(request_id) = input.replenishment_request_id.as_deref() {
            for (product_id, quantity) in received_by_product {
                let changed = transaction.execute(
                    "UPDATE replenishment_request_items
                     SET received_quantity = received_quantity + ?3,
                         version = version + 1
                     WHERE request_id = ?1 AND product_id = ?2
                       AND received_quantity + ?3 <= purchase_quantity",
                    params![request_id, product_id, quantity],
                )?;
                if changed != 1 {
                    return Err(AppError::Conflict(
                        "A quantidade recebida excede a compra aprovada ou o produto não pertence à reposição".into(),
                    ));
                }
            }
            refresh_replenishment_progress(&transaction, request_id, &now)?;
        }

        insert_audit(
            &transaction,
            "create_batch",
            "movement_batch",
            &batch_id,
            None,
            Some(
                serde_json::json!({"kind": input.kind, "reference": input.reference, "movementCount": movement_ids.len(), "replenishmentRequestId": input.replenishment_request_id}),
            ),
            &actor.id,
            &now,
        )?;
        transaction.commit()?;
        let movements = movement_ids
            .iter()
            .map(|id| {
                query_movement_by_id(&connection, id)?.ok_or_else(|| AppError::NotFound(id.clone()))
            })
            .collect::<AppResult<Vec<_>>>()?;
        Ok(MovementBatchView {
            id: batch_id,
            kind: input.kind,
            reference: input.reference.trim().into(),
            replenishment_request_id: input.replenishment_request_id,
            occurred_at: now,
            movements,
        })
    }

    pub fn create_asset_incident(
        &self,
        input: CreateAssetIncidentInput,
        actor: &AuthenticatedUser,
    ) -> AppResult<AssetIncidentView> {
        if !(10..=1000).contains(&input.description.trim().chars().count()) {
            return Err(AppError::Validation(
                "A descrição do problema deve ter entre 10 e 1000 caracteres".into(),
            ));
        }
        if input
            .custodian_name
            .as_deref()
            .is_some_and(|name| name.trim().chars().count() > 120)
        {
            return Err(AppError::Validation(
                "Nome do responsável muito longo".into(),
            ));
        }
        let mut connection = self.connection.lock();
        let transaction = connection.transaction()?;
        let (location_id, previous_status): (String, String) = transaction
            .query_row(
                "SELECT location_id, status FROM assets WHERE id = ?1 AND status <> 'disposed'",
                [&input.asset_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?
            .ok_or_else(|| AppError::NotFound(input.asset_id.clone()))?;
        ensure_actor_location(&transaction, actor, &location_id)?;
        let open: bool = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM asset_incidents WHERE asset_id = ?1 AND status NOT IN ('resolved', 'disposed'))",
            [&input.asset_id],
            |row| row.get(0),
        )?;
        if open {
            return Err(AppError::Conflict(
                "Este ativo já possui uma ocorrência aberta".into(),
            ));
        }
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        transaction.execute(
            "INSERT INTO asset_incidents(id, asset_id, reporter_id, custodian_name, description, reported_at) VALUES (?1, ?2, ?3, NULLIF(?4, ''), ?5, ?6)",
            params![id, input.asset_id, actor.id, input.custodian_name.as_deref().unwrap_or("").trim(), input.description.trim(), now],
        )?;
        transaction.execute(
            "UPDATE assets SET status = 'maintenance', version = version + 1, updated_at = ?2 WHERE id = ?1",
            params![input.asset_id, now],
        )?;
        insert_audit(
            &transaction,
            "report_incident",
            "asset_incident",
            &id,
            Some(serde_json::json!({"assetStatus": previous_status})),
            Some(
                serde_json::json!({"assetId": input.asset_id, "assetStatus": "maintenance", "custodianName": input.custodian_name, "description": input.description}),
            ),
            &actor.id,
            &now,
        )?;
        transaction.commit()?;
        query_asset_incident(&connection, &id)?.ok_or_else(|| AppError::NotFound(id))
    }

    pub fn list_asset_incidents(
        &self,
        actor: &AuthenticatedUser,
    ) -> AppResult<Vec<AssetIncidentView>> {
        let connection = self.connection.lock();
        let sql = if actor.role == UserRole::Admin {
            format!("{ASSET_INCIDENT_SELECT} ORDER BY ai.reported_at DESC")
        } else {
            format!("{ASSET_INCIDENT_SELECT} JOIN user_locations ul ON ul.location_id = a.location_id WHERE ul.user_id = ?1 ORDER BY ai.reported_at DESC")
        };
        let mut statement = connection.prepare(&sql)?;
        let rows = if actor.role == UserRole::Admin {
            statement
                .query_map([], map_asset_incident)?
                .collect::<Result<Vec<_>, _>>()?
        } else {
            statement
                .query_map([&actor.id], map_asset_incident)?
                .collect::<Result<Vec<_>, _>>()?
        };
        Ok(rows)
    }

    pub fn review_asset_incident(
        &self,
        input: ReviewAssetIncidentInput,
        actor: &AuthenticatedUser,
    ) -> AppResult<AssetIncidentView> {
        if !matches!(
            input.status.as_str(),
            "internal_repair" | "external_assistance" | "warranty" | "resolved" | "disposed"
        ) {
            return Err(AppError::Validation(
                "Decisão da ocorrência inválida".into(),
            ));
        }
        if input.resolution_note.trim().chars().count() < 10 {
            return Err(AppError::Validation(
                "Informe um parecer com ao menos 10 caracteres".into(),
            ));
        }
        let mut connection = self.connection.lock();
        let transaction = connection.transaction()?;
        let (asset_id, location_id): (String, String) = transaction.query_row(
            "SELECT ai.asset_id, a.location_id FROM asset_incidents ai JOIN assets a ON a.id = ai.asset_id WHERE ai.id = ?1 AND ai.version = ?2 AND ai.status NOT IN ('resolved', 'disposed')",
            params![input.id, input.version],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ).optional()?.ok_or_else(|| AppError::Conflict("A ocorrência foi alterada ou concluída".into()))?;
        ensure_actor_location(&transaction, actor, &location_id)?;
        let now = Utc::now().to_rfc3339();
        let terminal = matches!(input.status.as_str(), "resolved" | "disposed");
        transaction.execute(
            "UPDATE asset_incidents SET status = ?2, reviewer_id = ?3, resolution_note = ?4, reviewed_at = ?5, resolved_at = CASE WHEN ?6 THEN ?5 ELSE NULL END, version = version + 1 WHERE id = ?1",
            params![input.id, input.status, actor.id, input.resolution_note.trim(), now, terminal],
        )?;
        if terminal {
            let asset_status = if input.status == "resolved" {
                "available"
            } else {
                "disposed"
            };
            transaction.execute(
                "UPDATE assets SET status = ?2, version = version + 1, updated_at = ?3 WHERE id = ?1",
                params![asset_id, asset_status, now],
            )?;
        }
        insert_audit(
            &transaction,
            "review_incident",
            "asset_incident",
            &input.id,
            None,
            Some(serde_json::json!({"status": input.status, "note": input.resolution_note})),
            &actor.id,
            &now,
        )?;
        transaction.commit()?;
        query_asset_incident(&connection, &input.id)?.ok_or_else(|| AppError::NotFound(input.id))
    }

    pub fn create_adjustment_request(
        &self,
        input: CreateAdjustmentRequestInput,
        requester_id: &str,
    ) -> AppResult<AdjustmentRequestView> {
        validate_adjustment_request(&input)?;
        let mut connection = self.connection.lock();
        let transaction = connection.transaction()?;
        let now = Utc::now().to_rfc3339();
        let id = Uuid::new_v4().to_string();

        let (
            asset_id,
            location_id,
            requested_quantity,
            requested_status,
            requested_location_id,
            before_data,
            target_version,
        ) = match input.kind.as_str() {
            "quantity_adjustment" => {
                let location_id = required_value(&input.location_id, "Informe a unidade do saldo")?;
                ensure_location(&transaction, location_id)?;
                let requested_quantity = input.requested_quantity.ok_or_else(|| {
                    AppError::Validation("Informe a nova quantidade desejada".into())
                })?;
                let (tracking_type, version): (String, i64) = transaction
                    .query_row(
                        "SELECT tracking_type, version FROM products WHERE id = ?1 AND active = 1",
                        [&input.product_id],
                        |row| Ok((row.get(0)?, row.get(1)?)),
                    )
                    .optional()?
                    .ok_or_else(|| AppError::NotFound(input.product_id.clone()))?;
                if tracking_type != "quantity" {
                    return Err(AppError::Validation(
                        "Este produto exige uma solicitação de ativo patrimonial".into(),
                    ));
                }
                let current_quantity = get_balance(&transaction, &input.product_id, location_id)?;
                if current_quantity == requested_quantity {
                    return Err(AppError::Validation(
                        "A quantidade solicitada é igual ao saldo atual".into(),
                    ));
                }
                (
                    None,
                    Some(location_id.to_owned()),
                    Some(requested_quantity),
                    None,
                    None,
                    serde_json::json!({"quantity": current_quantity, "locationId": location_id}),
                    version,
                )
            }
            "asset_update" => {
                let asset_id = required_value(&input.asset_id, "Informe o ativo")?;
                let (product_id, location_id, status, version): (String, String, String, i64) =
                        transaction
                            .query_row(
                                "SELECT product_id, location_id, status, version FROM assets WHERE id = ?1",
                                [asset_id],
                                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
                            )
                            .optional()?
                            .ok_or_else(|| AppError::NotFound(asset_id.to_owned()))?;
                if product_id != input.product_id {
                    return Err(AppError::Validation(
                        "O ativo não pertence ao produto informado".into(),
                    ));
                }
                if status == "disposed" {
                    return Err(AppError::Validation(
                        "Um ativo baixado não pode ser ajustado".into(),
                    ));
                }
                if let Some(target) = input.requested_location_id.as_deref() {
                    ensure_location(&transaction, target)?;
                }
                if input.requested_status.as_deref() == Some(status.as_str())
                    && input.requested_location_id.as_deref() == Some(location_id.as_str())
                {
                    return Err(AppError::Validation(
                        "A alteração solicitada é igual ao estado atual".into(),
                    ));
                }
                (
                    Some(asset_id.to_owned()),
                    None,
                    None,
                    input.requested_status.clone(),
                    input.requested_location_id.clone(),
                    serde_json::json!({"locationId": location_id, "status": status}),
                    version,
                )
            }
            _ => unreachable!("validated adjustment kind"),
        };

        transaction.execute(
            "INSERT INTO adjustment_requests(id, requester_id, product_id, asset_id, kind, location_id, requested_quantity, requested_status, requested_location_id, description, before_data, target_version, requested_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![id, requester_id, input.product_id, asset_id, input.kind, location_id, requested_quantity, requested_status, requested_location_id, input.description.trim(), before_data.to_string(), target_version, now],
        )?;
        insert_audit(
            &transaction,
            "request",
            "adjustment_request",
            &id,
            None,
            Some(serde_json::json!({
                "requestId": id,
                "requesterId": requester_id,
                "productId": input.product_id,
                "assetId": asset_id,
                "kind": input.kind,
                "description": input.description.trim(),
                "before": before_data,
                "requestedQuantity": requested_quantity,
                "requestedStatus": requested_status,
                "requestedLocationId": requested_location_id,
            })),
            requester_id,
            &now,
        )?;
        transaction.commit()?;
        drop(connection);
        self.get_adjustment_request(&id)
    }

    pub fn create_adjustment_request_for(
        &self,
        input: CreateAdjustmentRequestInput,
        requester: &AuthenticatedUser,
    ) -> AppResult<AdjustmentRequestView> {
        if requester.role != UserRole::Admin {
            let connection = self.connection.lock();
            if let Some(location_id) = input.location_id.as_deref() {
                ensure_actor_location_connection(&connection, requester, location_id)?;
            }
            if let Some(asset_id) = input.asset_id.as_deref() {
                let current_location: String = connection
                    .query_row(
                        "SELECT location_id FROM assets WHERE id = ?1",
                        [asset_id],
                        |row| row.get(0),
                    )
                    .optional()?
                    .ok_or_else(|| AppError::NotFound(asset_id.to_owned()))?;
                ensure_actor_location_connection(&connection, requester, &current_location)?;
            }
            if let Some(location_id) = input.requested_location_id.as_deref() {
                ensure_actor_location_connection(&connection, requester, location_id)?;
            }
        }
        self.create_adjustment_request(input, &requester.id)
    }

    pub fn list_adjustment_requests(
        &self,
        viewer_id: &str,
        can_review: bool,
    ) -> AppResult<Vec<AdjustmentRequestView>> {
        let connection = self.connection.lock();
        let mut statement = connection.prepare(&format!(
            "{ADJUSTMENT_REQUEST_SELECT} WHERE (?1 = 1 OR ar.requester_id = ?2) ORDER BY CASE ar.status WHEN 'pending' THEN 0 ELSE 1 END, ar.requested_at DESC"
        ))?;
        let requests = statement
            .query_map(params![can_review, viewer_id], map_adjustment_request)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(requests)
    }

    pub fn list_adjustment_requests_for(
        &self,
        actor: &AuthenticatedUser,
        can_review: bool,
    ) -> AppResult<Vec<AdjustmentRequestView>> {
        let requests = self.list_adjustment_requests(&actor.id, can_review)?;
        if actor.role == UserRole::Admin || !can_review {
            return Ok(requests);
        }
        let connection = self.connection.lock();
        let allowed = query_user_location_ids(&connection, &actor.id)?
            .into_iter()
            .collect::<HashSet<_>>();
        Ok(requests
            .into_iter()
            .filter(|request| {
                let location = request.location_id.clone().or_else(|| {
                    request.asset_id.as_deref().and_then(|asset_id| {
                        connection
                            .query_row(
                                "SELECT location_id FROM assets WHERE id = ?1",
                                [asset_id],
                                |row| row.get::<_, String>(0),
                            )
                            .optional()
                            .ok()
                            .flatten()
                    })
                });
                location.is_some_and(|location| allowed.contains(&location))
            })
            .collect())
    }

    pub fn review_adjustment_request(
        &self,
        input: ReviewAdjustmentRequestInput,
        reviewer_id: &str,
    ) -> AppResult<AdjustmentRequestView> {
        validate_adjustment_review(&input)?;
        let mut connection = self.connection.lock();
        let transaction = connection.transaction()?;
        let request: AdjustmentRequestRecord = transaction
            .query_row(
                "SELECT requester_id, product_id, asset_id, kind, location_id, requested_quantity, requested_status, requested_location_id, status, target_version, version FROM adjustment_requests WHERE id = ?1",
                [&input.id],
                |row| {
                    Ok(AdjustmentRequestRecord {
                        requester_id: row.get(0)?,
                        product_id: row.get(1)?,
                        asset_id: row.get(2)?,
                        kind: row.get(3)?,
                        location_id: row.get(4)?,
                        requested_quantity: row.get(5)?,
                        requested_status: row.get(6)?,
                        requested_location_id: row.get(7)?,
                        status: row.get(8)?,
                        target_version: row.get(9)?,
                        version: row.get(10)?,
                    })
                },
            )
            .optional()?
            .ok_or_else(|| AppError::NotFound(input.id.clone()))?;
        if request.status != "pending" || request.version != input.version {
            return Err(AppError::Conflict(
                "A solicitação já foi analisada ou atualizada. Recarregue a lista".into(),
            ));
        }
        if request.requester_id == reviewer_id {
            return Err(AppError::Forbidden(
                "O solicitante não pode revisar a própria solicitação".into(),
            ));
        }

        let now = Utc::now().to_rfc3339();
        if input.decision == "approved" {
            apply_approved_adjustment(
                &transaction,
                &request,
                reviewer_id,
                &input.id,
                &input.note,
                &now,
            )?;
        }
        let updated = transaction.execute(
            "UPDATE adjustment_requests SET status = ?2, reviewer_id = ?3, review_note = ?4, reviewed_at = ?5, version = version + 1 WHERE id = ?1 AND status = 'pending' AND version = ?6",
            params![input.id, input.decision, reviewer_id, input.note.trim(), now, input.version],
        )?;
        if updated != 1 {
            return Err(AppError::Conflict(
                "A solicitação foi analisada por outro usuário".into(),
            ));
        }
        insert_audit(
            &transaction,
            if input.decision == "approved" {
                "approve"
            } else {
                "reject"
            },
            "adjustment_request",
            &input.id,
            Some(serde_json::json!({"status": "pending", "requesterId": request.requester_id})),
            Some(serde_json::json!({
                "requestId": input.id,
                "status": input.decision,
                "requesterId": request.requester_id,
                "reviewerId": reviewer_id,
                "reviewNote": input.note.trim(),
                "productId": request.product_id,
                "assetId": request.asset_id,
            })),
            reviewer_id,
            &now,
        )?;
        transaction.commit()?;
        drop(connection);
        self.get_adjustment_request(&input.id)
    }

    pub fn review_adjustment_request_for(
        &self,
        input: ReviewAdjustmentRequestInput,
        reviewer: &AuthenticatedUser,
    ) -> AppResult<AdjustmentRequestView> {
        if reviewer.role != UserRole::Admin {
            let connection = self.connection.lock();
            let location_id: Option<String> = connection
                .query_row(
                    "SELECT COALESCE(ar.location_id, a.location_id)
                     FROM adjustment_requests ar
                     LEFT JOIN assets a ON a.id = ar.asset_id
                     WHERE ar.id = ?1",
                    [&input.id],
                    |row| row.get(0),
                )
                .optional()?
                .flatten();
            let location_id = location_id.ok_or_else(|| AppError::NotFound(input.id.clone()))?;
            ensure_actor_location_connection(&connection, reviewer, &location_id)?;
        }
        self.review_adjustment_request(input, &reviewer.id)
    }

    fn get_adjustment_request(&self, id: &str) -> AppResult<AdjustmentRequestView> {
        let connection = self.connection.lock();
        connection
            .query_row(
                &format!("{ADJUSTMENT_REQUEST_SELECT} WHERE ar.id = ?1"),
                [id],
                map_adjustment_request,
            )
            .optional()?
            .ok_or_else(|| AppError::NotFound(id.to_owned()))
    }

    pub fn create_replenishment_request(
        &self,
        input: CreateReplenishmentRequestInput,
        requester: &AuthenticatedUser,
    ) -> AppResult<ReplenishmentRequestView> {
        validate_replenishment_request(&input)?;
        let mut connection = self.connection.lock();
        let transaction = connection.transaction()?;
        ensure_actor_location(&transaction, requester, &input.destination_location_id)?;

        let request_id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        transaction.execute(
            "INSERT INTO replenishment_requests(
                id, requester_id, destination_location_id, priority, justification, requested_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                request_id,
                requester.id,
                input.destination_location_id,
                input.priority,
                input.justification.trim(),
                now
            ],
        )?;

        for item in &input.items {
            let active: bool = transaction.query_row(
                "SELECT EXISTS(SELECT 1 FROM products WHERE id = ?1 AND active = 1)",
                [&item.product_id],
                |row| row.get(0),
            )?;
            if !active {
                return Err(AppError::NotFound(item.product_id.clone()));
            }
            let snapshot = get_product_stock_at_location(
                &transaction,
                &item.product_id,
                &input.destination_location_id,
            )?;
            transaction.execute(
                "INSERT INTO replenishment_request_items(
                    id, request_id, product_id, requested_quantity, stock_snapshot
                 ) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    Uuid::new_v4().to_string(),
                    request_id,
                    item.product_id,
                    item.quantity,
                    snapshot
                ],
            )?;
        }
        insert_audit(
            &transaction,
            "request",
            "replenishment_request",
            &request_id,
            None,
            Some(serde_json::json!({
                "requestId": request_id,
                "requesterId": requester.id,
                "destinationLocationId": input.destination_location_id,
                "priority": input.priority,
                "itemCount": input.items.len(),
                "justification": input.justification.trim()
            })),
            &requester.id,
            &now,
        )?;
        transaction.commit()?;
        drop(connection);
        self.get_replenishment_request(&request_id)
    }

    pub fn list_replenishment_requests(
        &self,
        actor: &AuthenticatedUser,
        can_review: bool,
    ) -> AppResult<Vec<ReplenishmentRequestView>> {
        let connection = self.connection.lock();
        let admin = actor.role == UserRole::Admin;
        let mut statement = connection.prepare(
            "SELECT rr.id
             FROM replenishment_requests rr
             WHERE ?1 = 1
                OR rr.requester_id = ?2
                OR EXISTS(
                    SELECT 1 FROM user_locations ul
                    WHERE ul.user_id = ?2 AND ul.location_id = rr.destination_location_id
                )
                OR (?3 = 1 AND EXISTS(
                    SELECT 1 FROM replenishment_request_items rri
                    JOIN user_locations ul ON ul.location_id = rri.source_location_id
                    WHERE rri.request_id = rr.id AND ul.user_id = ?2
                ))
             ORDER BY CASE rr.status WHEN 'pending' THEN 0 ELSE 1 END, rr.requested_at DESC",
        )?;
        let ids = statement
            .query_map(params![admin, actor.id, can_review], |row| {
                row.get::<_, String>(0)
            })?
            .collect::<Result<Vec<_>, _>>()?;
        drop(statement);
        ids.iter()
            .map(|id| get_replenishment_request_with_connection(&connection, id))
            .collect()
    }

    pub fn review_replenishment_request(
        &self,
        input: ReviewReplenishmentRequestInput,
        reviewer: &AuthenticatedUser,
    ) -> AppResult<ReplenishmentRequestView> {
        validate_replenishment_review(&input)?;
        let mut connection = self.connection.lock();
        let transaction = connection.transaction()?;
        let (requester_id, destination_id, status, version): (String, String, String, i64) =
            transaction
                .query_row(
                    "SELECT requester_id, destination_location_id, status, version
                     FROM replenishment_requests WHERE id = ?1",
                    [&input.id],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
                )
                .optional()?
                .ok_or_else(|| AppError::NotFound(input.id.clone()))?;
        if status != "pending" || version != input.version {
            return Err(AppError::Conflict(
                "A solicitação já foi analisada ou atualizada. Recarregue a lista".into(),
            ));
        }
        if requester_id == reviewer.id {
            return Err(AppError::Forbidden(
                "O solicitante não pode revisar a própria solicitação".into(),
            ));
        }
        ensure_actor_location(&transaction, reviewer, &destination_id)?;

        let mut statement = transaction.prepare(
            "SELECT rri.id, rri.product_id, rri.requested_quantity, p.tracking_type
             FROM replenishment_request_items rri
             JOIN products p ON p.id = rri.product_id
             WHERE rri.request_id = ?1",
        )?;
        let stored_items = statement
            .query_map([&input.id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        drop(statement);
        let stored = stored_items
            .into_iter()
            .map(|item| (item.0.clone(), item))
            .collect::<HashMap<_, _>>();
        if stored.len() != input.items.len() {
            return Err(AppError::Validation(
                "A decisão deve informar todos os itens da solicitação".into(),
            ));
        }

        let mut approved_total = 0_i64;
        let mut requested_total = 0_i64;
        let mut seen = HashSet::new();
        let now = Utc::now().to_rfc3339();
        for decision in &input.items {
            if !seen.insert(decision.item_id.as_str()) {
                return Err(AppError::Validation(
                    "Há uma decisão de item duplicada".into(),
                ));
            }
            let (_, product_id, requested_quantity, tracking_type) = stored
                .get(&decision.item_id)
                .ok_or_else(|| AppError::Validation("Um item não pertence à solicitação".into()))?;
            let approved = decision.transfer_quantity + decision.purchase_quantity;
            if decision.transfer_quantity < 0
                || decision.purchase_quantity < 0
                || approved > *requested_quantity
            {
                return Err(AppError::Validation(
                    "As quantidades aprovadas devem ser válidas e não exceder o pedido".into(),
                ));
            }
            if decision.transfer_quantity > 0 {
                let source = required_value(
                    &decision.source_location_id,
                    "Informe a unidade de origem da transferência",
                )?;
                if source == destination_id {
                    return Err(AppError::Validation(
                        "A origem da transferência deve ser diferente do destino".into(),
                    ));
                }
                ensure_actor_location(&transaction, reviewer, source)?;
                if tracking_type == "quantity" {
                    let (quantity, reserved): (i64, i64) = transaction
                        .query_row(
                            "SELECT quantity, reserved_quantity FROM stock_balances
                             WHERE product_id = ?1 AND location_id = ?2",
                            params![product_id, source],
                            |row| Ok((row.get(0)?, row.get(1)?)),
                        )
                        .optional()?
                        .unwrap_or((0, 0));
                    if quantity - reserved < decision.transfer_quantity {
                        return Err(AppError::Conflict(format!(
                            "Saldo disponível insuficiente na origem para o item {}",
                            decision.item_id
                        )));
                    }
                    transaction.execute(
                        "UPDATE stock_balances SET reserved_quantity = reserved_quantity + ?3, updated_at = ?4
                         WHERE product_id = ?1 AND location_id = ?2",
                        params![product_id, source, decision.transfer_quantity, now],
                    )?;
                } else {
                    let mut assets = transaction.prepare(
                        "SELECT a.id FROM assets a
                         WHERE a.product_id = ?1 AND a.location_id = ?2 AND a.status = 'available'
                           AND a.in_transit_shipment_id IS NULL
                           AND NOT EXISTS(SELECT 1 FROM replenishment_transfer_asset_reservations r WHERE r.asset_id = a.id)
                         ORDER BY a.asset_tag LIMIT ?3",
                    )?;
                    let asset_ids = assets
                        .query_map(
                            params![product_id, source, decision.transfer_quantity],
                            |row| row.get::<_, String>(0),
                        )?
                        .collect::<Result<Vec<_>, _>>()?;
                    drop(assets);
                    if asset_ids.len() != decision.transfer_quantity as usize {
                        return Err(AppError::Conflict(format!(
                            "Ativos disponíveis insuficientes na origem para o item {}",
                            decision.item_id
                        )));
                    }
                    for asset_id in asset_ids {
                        transaction.execute(
                            "INSERT INTO replenishment_transfer_asset_reservations(request_item_id, asset_id, reserved_at) VALUES (?1, ?2, ?3)",
                            params![decision.item_id, asset_id, now],
                        )?;
                    }
                }
            }
            let item_status = if approved == 0 {
                "rejected"
            } else if approved == *requested_quantity {
                "approved"
            } else {
                "partially_approved"
            };
            transaction.execute(
                "UPDATE replenishment_request_items
                 SET approved_quantity = ?2, transfer_quantity = ?3, purchase_quantity = ?4,
                     source_location_id = ?5, purchase_reference = ?6, status = ?7, version = version + 1
                 WHERE id = ?1 AND status = 'pending'",
                params![
                    decision.item_id,
                    approved,
                    decision.transfer_quantity,
                    decision.purchase_quantity,
                    decision.source_location_id,
                    decision.purchase_reference.as_deref().map(str::trim),
                    item_status
                ],
            )?;
            approved_total += approved;
            requested_total += requested_quantity;
        }
        let request_status = if approved_total == 0 {
            "rejected"
        } else if approved_total == requested_total {
            "approved"
        } else {
            "partially_approved"
        };
        let updated = transaction.execute(
            "UPDATE replenishment_requests
             SET status = ?2, reviewer_id = ?3, review_note = ?4, reviewed_at = ?5, version = version + 1
             WHERE id = ?1 AND status = 'pending' AND version = ?6",
            params![input.id, request_status, reviewer.id, input.note.trim(), now, input.version],
        )?;
        if updated != 1 {
            return Err(AppError::Conflict(
                "A solicitação foi analisada por outro usuário".into(),
            ));
        }
        insert_audit(
            &transaction,
            "review",
            "replenishment_request",
            &input.id,
            Some(serde_json::json!({"status": "pending", "requesterId": requester_id})),
            Some(serde_json::json!({
                "status": request_status,
                "requesterId": requester_id,
                "reviewerId": reviewer.id,
                "reviewNote": input.note.trim(),
                "approvedQuantity": approved_total,
                "requestedQuantity": requested_total
            })),
            &reviewer.id,
            &now,
        )?;
        transaction.commit()?;
        drop(connection);
        self.get_replenishment_request(&input.id)
    }

    pub fn dispatch_replenishment_transfer(
        &self,
        input: DispatchReplenishmentTransferInput,
        actor: &AuthenticatedUser,
    ) -> AppResult<ReplenishmentTransferShipmentView> {
        if actor.role == UserRole::Operator {
            return Err(AppError::Forbidden(
                "Somente gestores e administradores podem despachar transferências".into(),
            ));
        }
        validate_transfer_dispatch(&input)?;
        let mut connection = self.connection.lock();
        let transaction = connection.transaction()?;
        let (destination_id, request_status): (String, String) = transaction
            .query_row(
                "SELECT destination_location_id, status FROM replenishment_requests WHERE id = ?1",
                [&input.request_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?
            .ok_or_else(|| AppError::NotFound(input.request_id.clone()))?;
        if !matches!(
            request_status.as_str(),
            "approved" | "partially_approved" | "in_fulfillment"
        ) {
            return Err(AppError::Conflict(
                "A reposição não está disponível para despacho".into(),
            ));
        }

        let now = Utc::now().to_rfc3339();
        let shipment_id = Uuid::new_v4().to_string();
        let batch_id = Uuid::new_v4().to_string();
        let first_item = input
            .items
            .first()
            .ok_or_else(|| AppError::Validation("O despacho está vazio".into()))?;
        let source_id: String = transaction
            .query_row(
                "SELECT source_location_id FROM replenishment_request_items WHERE id = ?1 AND request_id = ?2",
                params![first_item.request_item_id, input.request_id],
                |row| row.get(0),
            )
            .optional()?
            .ok_or_else(|| AppError::Validation("O primeiro item não possui uma origem válida".into()))?;
        ensure_actor_location(&transaction, actor, &source_id)?;
        transaction.execute(
            "INSERT INTO replenishment_transfer_shipments(
                id, request_id, source_location_id, destination_location_id, reference,
                dispatch_note, status, dispatched_by, dispatched_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'dispatched', ?7, ?8)",
            params![
                shipment_id,
                input.request_id,
                source_id,
                destination_id,
                input.reference.trim(),
                input.note.trim(),
                actor.id,
                now
            ],
        )?;
        transaction.execute(
            "INSERT INTO movement_batches(id, kind, reference, note, replenishment_request_id, actor_id, occurred_at)
             VALUES (?1, 'transfer', ?2, ?3, ?4, ?5, ?6)",
            params![batch_id, input.reference.trim(), input.note.trim(), input.request_id, actor.id, now],
        )?;
        let mut total = 0_i64;
        let mut seen_items = HashSet::new();
        let mut seen_assets = HashSet::new();

        for line in &input.items {
            if !seen_items.insert(line.request_item_id.as_str()) || line.quantity <= 0 {
                return Err(AppError::Validation(
                    "Informe itens únicos e quantidades positivas no despacho".into(),
                ));
            }
            let (product_id, tracking_type, source, transfer, received, in_transit):
                (String, String, Option<String>, i64, i64, i64) = transaction
                .query_row(
                    "SELECT rri.product_id, p.tracking_type, rri.source_location_id,
                            rri.transfer_quantity, rri.transfer_received_quantity, rri.transfer_in_transit_quantity
                     FROM replenishment_request_items rri
                     JOIN products p ON p.id = rri.product_id
                     WHERE rri.id = ?1 AND rri.request_id = ?2",
                    params![line.request_item_id, input.request_id],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?)),
                )
                .optional()?
                .ok_or_else(|| AppError::Validation("Um item não pertence à reposição".into()))?;
            let source = source.ok_or_else(|| {
                AppError::Conflict("O item não possui origem de transferência".into())
            })?;
            if source_id != source {
                return Err(AppError::Validation(
                    "Cada despacho deve conter itens de uma única unidade de origem".into(),
                ));
            }
            ensure_actor_location(&transaction, actor, &source)?;
            let remaining = transfer - received - in_transit;
            if line.quantity > remaining {
                return Err(AppError::Conflict(format!(
                    "A quantidade despachada excede o saldo pendente do item {}",
                    line.request_item_id
                )));
            }

            if tracking_type == "quantity" {
                if !line.asset_ids.is_empty() {
                    return Err(AppError::Validation(
                        "Produtos controlados por quantidade não aceitam ativos individuais".into(),
                    ));
                }
                let (quantity, reserved): (i64, i64) = transaction
                    .query_row(
                        "SELECT quantity, reserved_quantity FROM stock_balances WHERE product_id = ?1 AND location_id = ?2",
                        params![product_id, source],
                        |row| Ok((row.get(0)?, row.get(1)?)),
                    )
                    .optional()?
                    .unwrap_or((0, 0));
                if quantity < line.quantity || reserved < line.quantity {
                    return Err(AppError::Conflict(
                        "O saldo reservado não está mais disponível para despacho".into(),
                    ));
                }
                transaction.execute(
                    "UPDATE stock_balances SET quantity = quantity - ?3, reserved_quantity = reserved_quantity - ?3, updated_at = ?4
                     WHERE product_id = ?1 AND location_id = ?2",
                    params![product_id, source, line.quantity, now],
                )?;
                let shipment_item_id = Uuid::new_v4().to_string();
                transaction.execute(
                    "INSERT INTO replenishment_transfer_shipment_items(id, shipment_id, request_item_id, product_id, quantity)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![shipment_item_id, shipment_id, line.request_item_id, product_id, line.quantity],
                )?;
                let movement_id = Uuid::new_v4().to_string();
                transaction.execute(
                    "INSERT INTO stock_movements(id, product_id, kind, quantity, from_location_id, to_location_id, actor_id, note, occurred_at, batch_id)
                     VALUES (?1, ?2, 'transfer', ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    params![movement_id, product_id, line.quantity, source, destination_id, actor.id, input.note.trim(), now, batch_id],
                )?;
                insert_audit(
                    &transaction,
                    "dispatch_transfer",
                    "stock",
                    &product_id,
                    Some(serde_json::json!({"quantity": quantity, "reservedQuantity": reserved})),
                    Some(
                        serde_json::json!({"quantity": quantity - line.quantity, "reservedQuantity": reserved - line.quantity, "inTransit": line.quantity}),
                    ),
                    &actor.id,
                    &now,
                )?;
            } else {
                if line.asset_ids.len() != line.quantity as usize {
                    return Err(AppError::Validation(
                        "Selecione exatamente os ativos serializados deste despacho".into(),
                    ));
                }
                for asset_id in &line.asset_ids {
                    if !seen_assets.insert(asset_id.as_str()) {
                        return Err(AppError::Validation(
                            "Há um ativo duplicado no despacho".into(),
                        ));
                    }
                    let valid: bool = transaction.query_row(
                        "SELECT EXISTS(
                           SELECT 1 FROM assets a
                           JOIN replenishment_transfer_asset_reservations r ON r.asset_id = a.id
                           WHERE a.id = ?1 AND a.product_id = ?2 AND a.location_id = ?3
                             AND a.status = 'available' AND a.in_transit_shipment_id IS NULL
                             AND r.request_item_id = ?4
                         )",
                        params![asset_id, product_id, source, line.request_item_id],
                        |row| row.get(0),
                    )?;
                    if !valid {
                        return Err(AppError::Conflict(format!(
                            "O ativo {asset_id} não está reservado ou disponível para este despacho"
                        )));
                    }
                    transaction.execute(
                        "UPDATE assets SET in_transit_shipment_id = ?2, updated_at = ?3 WHERE id = ?1",
                        params![asset_id, shipment_id, now],
                    )?;
                    let shipment_item_id = Uuid::new_v4().to_string();
                    transaction.execute(
                        "INSERT INTO replenishment_transfer_shipment_items(id, shipment_id, request_item_id, product_id, asset_id, quantity)
                         VALUES (?1, ?2, ?3, ?4, ?5, 1)",
                        params![shipment_item_id, shipment_id, line.request_item_id, product_id, asset_id],
                    )?;
                    let movement_id = Uuid::new_v4().to_string();
                    transaction.execute(
                        "INSERT INTO stock_movements(id, product_id, asset_id, kind, quantity, from_location_id, to_location_id, actor_id, note, occurred_at, batch_id)
                         VALUES (?1, ?2, ?3, 'transfer', 1, ?4, ?5, ?6, ?7, ?8, ?9)",
                        params![movement_id, product_id, asset_id, source, destination_id, actor.id, input.note.trim(), now, batch_id],
                    )?;
                }
            }
            transaction.execute(
                "UPDATE replenishment_request_items
                 SET transfer_in_transit_quantity = transfer_in_transit_quantity + ?2,
                     status = 'in_fulfillment', version = version + 1 WHERE id = ?1",
                params![line.request_item_id, line.quantity],
            )?;
            total += line.quantity;
        }
        refresh_replenishment_progress(&transaction, &input.request_id, &now)?;
        insert_audit(
            &transaction,
            "dispatch_transfer",
            "replenishment_transfer_shipment",
            &shipment_id,
            None,
            Some(
                serde_json::json!({"requestId": input.request_id, "sourceLocationId": source_id, "destinationLocationId": destination_id, "reference": input.reference.trim(), "quantity": total}),
            ),
            &actor.id,
            &now,
        )?;
        transaction.commit()?;
        drop(connection);
        self.get_replenishment_request(&input.request_id)?
            .transfer_shipments
            .into_iter()
            .find(|shipment| shipment.id == shipment_id)
            .ok_or_else(|| AppError::NotFound(shipment_id))
    }

    pub fn receive_replenishment_transfer(
        &self,
        input: ReceiveReplenishmentTransferInput,
        actor: &AuthenticatedUser,
    ) -> AppResult<ReplenishmentTransferShipmentView> {
        validate_transfer_receipt(&input)?;
        let mut connection = self.connection.lock();
        let transaction = connection.transaction()?;
        let (request_id, source_id, destination_id, status, version): (
            String,
            String,
            String,
            String,
            i64,
        ) = transaction
            .query_row(
                "SELECT request_id, source_location_id, destination_location_id, status, version
                 FROM replenishment_transfer_shipments WHERE id = ?1",
                [&input.shipment_id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .optional()?
            .ok_or_else(|| AppError::NotFound(input.shipment_id.clone()))?;
        if !matches!(status.as_str(), "dispatched" | "partially_received")
            || version != input.version
        {
            return Err(AppError::Conflict(
                "O despacho já foi concluído ou atualizado. Recarregue a lista".into(),
            ));
        }
        ensure_actor_location(&transaction, actor, &destination_id)?;
        let now = Utc::now().to_rfc3339();
        let mut seen = HashSet::new();
        let mut received_total = 0_i64;
        let mut rejected_total = 0_i64;
        for line in &input.items {
            if !seen.insert(line.shipment_item_id.as_str())
                || line.received_quantity < 0
                || line.rejected_quantity < 0
                || line.received_quantity + line.rejected_quantity <= 0
            {
                return Err(AppError::Validation(
                    "Informe linhas válidas e únicas no recebimento".into(),
                ));
            }
            if line.rejected_quantity > 0 && line.note.trim().chars().count() < 5 {
                return Err(AppError::Validation(
                    "Descreva a divergência dos itens recusados com pelo menos 5 caracteres".into(),
                ));
            }
            let (request_item_id, product_id, asset_id, quantity, received, rejected):
                (String, String, Option<String>, i64, i64, i64) = transaction
                .query_row(
                    "SELECT request_item_id, product_id, asset_id, quantity, received_quantity, rejected_quantity
                     FROM replenishment_transfer_shipment_items WHERE id = ?1 AND shipment_id = ?2",
                    params![line.shipment_item_id, input.shipment_id],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?)),
                )
                .optional()?
                .ok_or_else(|| AppError::Validation("Uma linha não pertence ao despacho".into()))?;
            let processed = line.received_quantity + line.rejected_quantity;
            if processed > quantity - received - rejected || (asset_id.is_some() && processed != 1)
            {
                return Err(AppError::Conflict(
                    "A quantidade recebida excede o saldo em trânsito".into(),
                ));
            }
            if let Some(asset_id) = asset_id.as_deref() {
                if line.received_quantity == 1 {
                    transaction.execute(
                        "UPDATE assets SET location_id = ?2, in_transit_shipment_id = NULL, updated_at = ?3
                         WHERE id = ?1 AND in_transit_shipment_id = ?4",
                        params![asset_id, destination_id, now, input.shipment_id],
                    )?;
                    transaction.execute(
                        "DELETE FROM replenishment_transfer_asset_reservations WHERE asset_id = ?1",
                        [asset_id],
                    )?;
                } else {
                    transaction.execute(
                        "UPDATE assets SET in_transit_shipment_id = NULL, updated_at = ?2
                         WHERE id = ?1 AND in_transit_shipment_id = ?3",
                        params![asset_id, now, input.shipment_id],
                    )?;
                }
            } else {
                if line.received_quantity > 0 {
                    let destination_before =
                        get_balance(&transaction, &product_id, &destination_id)?;
                    set_balance(
                        &transaction,
                        &product_id,
                        &destination_id,
                        destination_before + line.received_quantity,
                        &now,
                    )?;
                }
                if line.rejected_quantity > 0 {
                    transaction.execute(
                        "INSERT INTO stock_balances(product_id, location_id, quantity, reserved_quantity, updated_at)
                         VALUES (?1, ?2, ?3, ?3, ?4)
                         ON CONFLICT(product_id, location_id) DO UPDATE SET
                           quantity = quantity + excluded.quantity,
                           reserved_quantity = reserved_quantity + excluded.reserved_quantity,
                           updated_at = excluded.updated_at",
                        params![product_id, source_id, line.rejected_quantity, now],
                    )?;
                }
            }
            transaction.execute(
                "UPDATE replenishment_transfer_shipment_items
                 SET received_quantity = received_quantity + ?2,
                     rejected_quantity = rejected_quantity + ?3,
                     receipt_note = CASE WHEN trim(?4) = '' THEN receipt_note ELSE trim(?4) END
                 WHERE id = ?1",
                params![
                    line.shipment_item_id,
                    line.received_quantity,
                    line.rejected_quantity,
                    line.note
                ],
            )?;
            transaction.execute(
                "UPDATE replenishment_request_items
                 SET transfer_in_transit_quantity = transfer_in_transit_quantity - ?2,
                     transfer_received_quantity = transfer_received_quantity + ?3,
                     transfer_rejected_quantity = transfer_rejected_quantity + ?4,
                     version = version + 1 WHERE id = ?1",
                params![
                    request_item_id,
                    processed,
                    line.received_quantity,
                    line.rejected_quantity
                ],
            )?;
            received_total += line.received_quantity;
            rejected_total += line.rejected_quantity;
        }
        let (remaining, rejected_all): (i64, i64) = transaction.query_row(
            "SELECT COALESCE(SUM(quantity - received_quantity - rejected_quantity), 0),
                    COALESCE(SUM(rejected_quantity), 0)
             FROM replenishment_transfer_shipment_items WHERE shipment_id = ?1",
            [&input.shipment_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        let shipment_status = if remaining > 0 {
            "partially_received"
        } else if rejected_all > 0 {
            "divergent"
        } else {
            "received"
        };
        let updated = transaction.execute(
            "UPDATE replenishment_transfer_shipments
             SET receipt_reference = ?2, receipt_note = ?3, status = ?4, received_by = ?5,
                 received_at = ?6, version = version + 1
             WHERE id = ?1 AND version = ?7",
            params![
                input.shipment_id,
                input.reference.trim(),
                input.note.trim(),
                shipment_status,
                actor.id,
                now,
                input.version
            ],
        )?;
        if updated != 1 {
            return Err(AppError::Conflict(
                "O despacho foi recebido por outro usuário".into(),
            ));
        }
        refresh_replenishment_progress(&transaction, &request_id, &now)?;
        insert_audit(
            &transaction,
            "receive_transfer",
            "replenishment_transfer_shipment",
            &input.shipment_id,
            Some(serde_json::json!({"status": status, "version": version})),
            Some(
                serde_json::json!({"status": shipment_status, "receivedQuantity": received_total, "rejectedQuantity": rejected_total, "reference": input.reference.trim()}),
            ),
            &actor.id,
            &now,
        )?;
        transaction.commit()?;
        drop(connection);
        self.get_replenishment_request(&request_id)?
            .transfer_shipments
            .into_iter()
            .find(|shipment| shipment.id == input.shipment_id)
            .ok_or_else(|| AppError::NotFound(input.shipment_id))
    }

    fn get_replenishment_request(&self, id: &str) -> AppResult<ReplenishmentRequestView> {
        let connection = self.connection.lock();
        get_replenishment_request_with_connection(&connection, id)
    }

    fn save_master_data(
        &self,
        kind: MasterDataKind,
        input: SaveMasterDataInput,
        actor_id: &str,
    ) -> AppResult<MasterDataRecord> {
        validate_master_data(&input)?;
        let mut connection = self.connection.lock();
        let transaction = connection.transaction()?;
        let table = master_data_table(kind);
        let entity_type = master_data_entity(kind);
        let now = Utc::now().to_rfc3339();
        let id = input
            .id
            .clone()
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        let before = if input.id.is_some() {
            let sql = format!("SELECT json_object('name', name, 'code', code, 'active', active) FROM {table} WHERE id = ?1");
            Some(
                transaction
                    .query_row(&sql, [&id], |row| row.get::<_, String>(0))
                    .optional()?
                    .ok_or_else(|| AppError::NotFound(id.clone()))?,
            )
        } else {
            None
        };

        if input.id.is_some() {
            let sql =
                format!("UPDATE {table} SET name = ?2, code = ?3, updated_at = ?4 WHERE id = ?1");
            transaction.execute(
                &sql,
                params![id, input.name.trim(), normalize_code(&input.code), now],
            )?;
        } else {
            let sql = format!("INSERT INTO {table}(id, name, code, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?4)");
            transaction.execute(
                &sql,
                params![id, input.name.trim(), normalize_code(&input.code), now],
            )?;
        }
        insert_audit(
            &transaction,
            if input.id.is_some() {
                "update"
            } else {
                "create"
            },
            entity_type,
            &id,
            before.and_then(|value| serde_json::from_str(&value).ok()),
            Some(
                serde_json::json!({"name": input.name.trim(), "code": normalize_code(&input.code), "active": true}),
            ),
            actor_id,
            &now,
        )?;
        transaction.commit()?;
        drop(connection);
        self.get_master_data(kind, &id)
    }

    fn set_master_data_active(
        &self,
        kind: MasterDataKind,
        id: &str,
        active: bool,
        actor_id: &str,
    ) -> AppResult<MasterDataRecord> {
        let mut connection = self.connection.lock();
        let transaction = connection.transaction()?;
        let current = query_master_data_record(&transaction, kind, id)?
            .ok_or_else(|| AppError::NotFound(id.to_owned()))?;
        if !active && current.usage_count > 0 {
            let noun = match kind {
                MasterDataKind::Location => "A unidade ainda possui estoque ou ativos",
                MasterDataKind::Category => "A categoria ainda possui produtos vinculados",
            };
            return Err(AppError::Validation(format!(
                "{noun}. Remova ou transfira os vínculos antes de desativar"
            )));
        }
        if current.active == active {
            return Ok(current);
        }
        let table = master_data_table(kind);
        let now = Utc::now().to_rfc3339();
        let sql = format!("UPDATE {table} SET active = ?2, updated_at = ?3 WHERE id = ?1");
        transaction.execute(&sql, params![id, active, now])?;
        insert_audit(
            &transaction,
            if active { "activate" } else { "deactivate" },
            master_data_entity(kind),
            id,
            Some(serde_json::json!({"active": current.active})),
            Some(serde_json::json!({"active": active})),
            actor_id,
            &now,
        )?;
        transaction.commit()?;
        drop(connection);
        self.get_master_data(kind, id)
    }

    fn get_master_data(&self, kind: MasterDataKind, id: &str) -> AppResult<MasterDataRecord> {
        let connection = self.connection.lock();
        query_master_data_record(&connection, kind, id)?
            .ok_or_else(|| AppError::NotFound(id.to_owned()))
    }

    fn get_item(&self, id: &str) -> AppResult<InventoryItem> {
        let connection = self.connection.lock();
        query_items(&connection, "", false)?
            .into_iter()
            .find(|item| item.id == id)
            .ok_or_else(|| AppError::NotFound(id.to_owned()))
    }

    fn get_movement(&self, id: &str) -> AppResult<MovementView> {
        let connection = self.connection.lock();
        query_movement_by_id(&connection, id)?.ok_or_else(|| AppError::NotFound(id.to_owned()))
    }

    fn get_asset_record(&self, id: &str) -> AppResult<AssetRecordView> {
        let connection = self.connection.lock();
        query_asset_record_by_id(&connection, id)?.ok_or_else(|| AppError::NotFound(id.to_owned()))
    }

    pub fn create_backup(&self) -> AppResult<BackupRecord> {
        self.create_backup_with_kind("manual")
    }

    pub fn import_context(&self, location_id: &str) -> AppResult<ImportContext> {
        let connection = self.connection.lock();
        let location_exists: bool = connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM locations WHERE id = ?1 AND active = 1)",
            [location_id],
            |row| row.get(0),
        )?;
        if !location_exists {
            return Err(AppError::Validation("Selecione uma unidade ativa".into()));
        }
        Ok(ImportContext {
            existing_skus: query_normalized_values(&connection, "SELECT sku FROM products")?,
            existing_asset_tags: query_normalized_values(
                &connection,
                "SELECT asset_tag FROM assets",
            )?,
            existing_serial_numbers: query_normalized_values(
                &connection,
                "SELECT serial_number FROM assets WHERE serial_number IS NOT NULL",
            )?,
        })
    }

    pub fn import_stage(&self, stage: StagedImport, actor_id: &str) -> AppResult<ImportResult> {
        if stage.products.is_empty() {
            return Err(AppError::Validation(
                "A prévia não contém produtos válidos".into(),
            ));
        }
        let safety = self.create_backup_with_kind("import")?;
        let mut connection = self.connection.lock();
        let transaction = connection.transaction()?;
        ensure_location(&transaction, &stage.location_id)?;
        let imported_at = Utc::now().to_rfc3339();
        let mut assets_created = 0_usize;
        let mut quantity_imported = 0_i64;

        for product in &stage.products {
            let duplicate: bool = transaction.query_row(
                "SELECT EXISTS(SELECT 1 FROM products WHERE sku = ?1 COLLATE NOCASE)",
                [&product.sku],
                |row| row.get(0),
            )?;
            if duplicate {
                return Err(AppError::Validation(format!(
                    "O SKU {} foi cadastrado depois da prévia; analise o arquivo novamente",
                    product.sku
                )));
            }
            let product_id = Uuid::new_v4().to_string();
            let category_id = find_or_create_category(&transaction, &product.category)?;
            transaction.execute(
                "INSERT INTO products(id, sku, name, category_id, tracking_type, minimum_quantity, serial_number_policy, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, CASE WHEN ?5 = 'serialized' THEN 'required' ELSE 'not_applicable' END, ?7, ?7)",
                params![product_id, product.sku, product.name, category_id, product.tracking_type, product.minimum_quantity, imported_at],
            )?;
            if product.tracking_type == "quantity" {
                if product.quantity > 0 {
                    set_balance(
                        &transaction,
                        &product_id,
                        &stage.location_id,
                        product.quantity,
                        &imported_at,
                    )?;
                    insert_movement(
                        &transaction,
                        &product_id,
                        None,
                        "entry",
                        product.quantity,
                        None,
                        Some(&stage.location_id),
                        "Importação de planilha",
                        actor_id,
                        &imported_at,
                    )?;
                    quantity_imported += product.quantity;
                }
            } else {
                for asset in &product.assets {
                    let asset_id = Uuid::new_v4().to_string();
                    transaction.execute(
                        "INSERT INTO assets(id, product_id, asset_tag, serial_number, location_id, status, created_at, updated_at) VALUES (?1, ?2, ?3, NULLIF(?4, ''), ?5, 'available', ?6, ?6)",
                        params![asset_id, product_id, asset.asset_tag, asset.serial_number.as_deref().unwrap_or_default(), stage.location_id, imported_at],
                    )?;
                    insert_movement(
                        &transaction,
                        &product_id,
                        Some(&asset_id),
                        "entry",
                        1,
                        None,
                        Some(&stage.location_id),
                        "Importação de planilha",
                        actor_id,
                        &imported_at,
                    )?;
                    assets_created += 1;
                    quantity_imported += 1;
                }
            }
        }

        insert_audit(
            &transaction,
            "import",
            "spreadsheet",
            &stage.file_name,
            None,
            Some(serde_json::json!({
                "fileName": stage.file_name,
                "productsCreated": stage.products.len(),
                "assetsCreated": assets_created,
                "quantityImported": quantity_imported,
                "destinationLocationId": stage.location_id,
                "safetyBackup": safety.file_name,
            })),
            actor_id,
            &imported_at,
        )?;
        transaction.commit()?;
        Ok(ImportResult {
            file_name: stage.file_name,
            products_created: stage.products.len(),
            assets_created,
            quantity_imported,
            safety_backup: safety.file_name,
            imported_at,
        })
    }

    pub fn list_backups(&self) -> AppResult<Vec<BackupRecord>> {
        let directory = self.backup_directory()?;
        if !directory.exists() {
            return Ok(Vec::new());
        }
        let mut backups = fs::read_dir(directory)?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|extension| extension == "db"))
            .map(|path| backup_record(&path, &self.storage_key))
            .collect::<AppResult<Vec<_>>>()?;
        backups.sort_by(|left, right| right.created_at.cmp(&left.created_at));
        Ok(backups)
    }

    pub fn restore_backup(&self, file_name: &str, actor_id: &str) -> AppResult<RestoreResult> {
        let source_path = self.resolve_backup(file_name)?;
        validate_backup_file(&source_path, &self.storage_key)?;
        let safety = self.create_backup_with_kind("safety")?;
        let source = open_encrypted_read_only(&source_path, &self.storage_key)?;
        let mut destination = self.connection.lock();
        {
            let backup = Backup::new(&source, &mut destination)?;
            backup.run_to_completion(64, Duration::from_millis(5), None)?;
        }
        destination.execute_batch(
            "PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL; PRAGMA busy_timeout = 5000;",
        )?;
        validate_connection(&destination)?;
        let restored_at = Utc::now().to_rfc3339();
        let transaction = destination.transaction()?;
        insert_audit(
            &transaction,
            "restore",
            "backup",
            file_name,
            Some(serde_json::json!({"safetyBackup": safety.file_name})),
            Some(serde_json::json!({"restoredFrom": file_name})),
            actor_id,
            &restored_at,
        )?;
        transaction.commit()?;
        Ok(RestoreResult {
            restored_from: file_name.to_owned(),
            safety_backup: safety.file_name,
            restored_at,
        })
    }

    fn create_backup_with_kind(&self, kind: &str) -> AppResult<BackupRecord> {
        let directory = self.backup_directory()?;
        fs::create_dir_all(&directory)?;
        let prefix = match kind {
            "safety" => "pre-restore",
            "import" => "pre-import",
            _ => "stockmanager",
        };
        let unique = Uuid::new_v4().to_string();
        let filename = format!(
            "{}-{}-{}.db",
            prefix,
            Utc::now().format("%Y%m%d-%H%M%S-%3f"),
            &unique[..8]
        );
        let destination = directory.join(filename);
        self.backup_to(&destination)?;
        backup_record(&destination, &self.storage_key)
    }

    fn backup_directory(&self) -> AppResult<PathBuf> {
        self.path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .map(|parent| parent.join("backups"))
            .ok_or_else(|| AppError::Validation("Backup indisponível para banco em memória".into()))
    }

    fn resolve_backup(&self, file_name: &str) -> AppResult<PathBuf> {
        let supplied = Path::new(file_name);
        if supplied.file_name().and_then(|name| name.to_str()) != Some(file_name)
            || supplied.extension().and_then(|value| value.to_str()) != Some("db")
        {
            return Err(AppError::Validation("Nome de backup inválido".into()));
        }
        let directory = self.backup_directory()?;
        let canonical_directory = directory.canonicalize()?;
        let candidate = directory.join(file_name).canonicalize()?;
        if candidate.parent() != Some(canonical_directory.as_path()) {
            return Err(AppError::Validation(
                "O backup deve pertencer à pasta interna da aplicação".into(),
            ));
        }
        Ok(candidate)
    }

    fn backup_to(&self, destination: &Path) -> AppResult<()> {
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        if destination.exists() {
            return Err(AppError::Conflict("O arquivo de backup já existe".into()));
        }
        let source = self.connection.lock();
        let mut encrypted_destination = Connection::open(destination)?;
        unlock(&encrypted_destination, &self.storage_key)?;
        {
            let backup = Backup::new(&source, &mut encrypted_destination)?;
            backup.run_to_completion(64, Duration::from_millis(5), None)?;
        }
        validate_connection(&encrypted_destination)?;
        Ok(())
    }
}

fn open_or_encrypt_database(path: &Path, key: &StorageKey) -> AppResult<Connection> {
    if path == Path::new(":memory:") || !path.exists() || fs::metadata(path)?.len() == 0 {
        return open_encrypted(path, key);
    }

    match open_encrypted(path, key) {
        Ok(connection) => Ok(connection),
        Err(_) if is_plaintext_database(path) => {
            encrypt_plaintext_database(path, key)?;
            open_encrypted(path, key)
        }
        Err(_) => Err(AppError::Security(
            "O banco está criptografado, mas a chave deste computador não o desbloqueou. Não crie um novo banco; restaure a chave de recuperação correspondente"
                .into(),
        )),
    }
}

#[cfg(test)]
fn resolve_storage_key(_path: &Path) -> AppResult<StorageKey> {
    Ok(StorageKey::generate())
}

#[cfg(not(test))]
fn resolve_storage_key(path: &Path) -> AppResult<StorageKey> {
    let populated = path.exists() && fs::metadata(path)?.len() > 0;
    let plaintext = populated && is_plaintext_database(path);
    let recovery_path = path.with_file_name("stockmanager.recovery-key");
    let stored = StorageKey::load()?;

    if let Some(key) = stored {
        if !populated || plaintext || open_encrypted(path, &key).is_ok() {
            return Ok(key);
        }
        if recovery_path.exists() {
            return import_recovery_key(path, &recovery_path);
        }
        return Err(AppError::Security(
            "A chave do cofre do Windows não corresponde ao banco. Coloque a chave de recuperação no arquivo 'stockmanager.recovery-key' ao lado do banco e abra o sistema novamente"
                .into(),
        ));
    }

    if recovery_path.exists() {
        return import_recovery_key(path, &recovery_path);
    }
    if populated && !plaintext {
        return Err(AppError::Security(
            "A chave do banco não está neste computador. Coloque a chave de recuperação no arquivo 'stockmanager.recovery-key' ao lado do banco e abra o sistema novamente"
                .into(),
        ));
    }

    let key = StorageKey::generate();
    key.persist()?;
    Ok(key)
}

#[cfg(not(test))]
fn import_recovery_key(database_path: &Path, recovery_path: &Path) -> AppResult<StorageKey> {
    let raw = zeroize::Zeroizing::new(fs::read_to_string(recovery_path)?);
    let key = StorageKey::from_hex(raw.trim().to_owned())?;
    if database_path.exists()
        && fs::metadata(database_path)?.len() > 0
        && !is_plaintext_database(database_path)
    {
        open_encrypted(database_path, &key).map_err(|_| {
            AppError::Security("A chave fornecida não desbloqueia este banco".into())
        })?;
    }
    key.persist()?;
    fs::remove_file(recovery_path)?;
    Ok(key)
}

fn is_plaintext_database(path: &Path) -> bool {
    Connection::open(path)
        .and_then(|connection| {
            connection.query_row("SELECT COUNT(*) FROM sqlite_master", [], |row| {
                row.get::<_, i64>(0)
            })
        })
        .is_ok()
}

fn encrypt_plaintext_database(path: &Path, key: &StorageKey) -> AppResult<()> {
    let source = Connection::open(path)?;
    source.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;

    let temporary_path = path.with_extension(format!("encrypted-{}.tmp", Uuid::new_v4()));
    let escaped_path = temporary_path.to_string_lossy().replace('\'', "''");
    let export = format!(
        "ATTACH DATABASE '{escaped_path}' AS encrypted KEY \"x'{}'\";\
         PRAGMA encrypted.cipher_page_size = 4096;\
         SELECT sqlcipher_export('encrypted');\
         DETACH DATABASE encrypted;",
        key.expose()
    );
    if let Err(error) = source.execute_batch(&export) {
        let _ = fs::remove_file(&temporary_path);
        return Err(error.into());
    }
    drop(source);

    let encrypted = open_encrypted(&temporary_path, key)?;
    let integrity: String = encrypted.query_row("PRAGMA quick_check", [], |row| row.get(0))?;
    if integrity != "ok" {
        drop(encrypted);
        let _ = fs::remove_file(&temporary_path);
        return Err(AppError::Security(format!(
            "A conversão criptográfica falhou na verificação de integridade: {integrity}"
        )));
    }
    drop(encrypted);

    let rollback_path = path.with_extension(format!("plaintext-{}.rollback", Uuid::new_v4()));
    fs::rename(path, &rollback_path)?;
    if let Err(error) = fs::rename(&temporary_path, path) {
        let _ = fs::rename(&rollback_path, path);
        return Err(error.into());
    }
    let _ = fs::remove_file(path.with_extension("db-wal"));
    let _ = fs::remove_file(path.with_extension("db-shm"));
    fs::remove_file(rollback_path)?;
    Ok(())
}

fn apply_migration(connection: &Connection, version: i64, migration: &str) -> AppResult<()> {
    let already_applied = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version = ?1)",
        [version],
        |row| row.get::<_, bool>(0),
    )?;
    if !already_applied {
        connection.execute_batch(migration)?;
    }
    Ok(())
}

fn backup_record(path: &Path, key: &StorageKey) -> AppResult<BackupRecord> {
    let metadata = fs::metadata(path)?;
    let modified: DateTime<Utc> = metadata
        .modified()
        .map(DateTime::<Utc>::from)
        .unwrap_or_else(|_| Utc::now());
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| AppError::Validation("Nome de backup inválido".into()))?
        .to_owned();
    Ok(BackupRecord {
        kind: if file_name.starts_with("pre-restore-") {
            "safety"
        } else if file_name.starts_with("pre-import-") {
            "import"
        } else {
            "manual"
        }
        .into(),
        file_name,
        created_at: modified.to_rfc3339(),
        size_bytes: metadata.len(),
        valid: validate_backup_file(path, key).is_ok(),
    })
}

fn query_normalized_values(connection: &Connection, sql: &str) -> AppResult<HashSet<String>> {
    let mut statement = connection.prepare(sql)?;
    let values = statement
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(values
        .into_iter()
        .map(|value| normalize_key(&value))
        .collect())
}

fn validate_backup_file(path: &Path, key: &StorageKey) -> AppResult<()> {
    let connection = open_encrypted_read_only(path, key)?;
    validate_connection(&connection)
}

fn validate_connection(connection: &Connection) -> AppResult<()> {
    let integrity: String = connection.query_row("PRAGMA quick_check", [], |row| row.get(0))?;
    if integrity != "ok" {
        return Err(AppError::Validation(format!(
            "Falha na integridade do backup: {integrity}"
        )));
    }
    let required_tables: i64 = connection.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name IN ('schema_migrations', 'users', 'products', 'stock_balances', 'assets', 'stock_movements', 'audit_logs')",
        [],
        |row| row.get(0),
    )?;
    if required_tables != 7 {
        return Err(AppError::Validation(
            "O arquivo não contém a estrutura obrigatória do StockManager Pro".into(),
        ));
    }
    let version: i64 = connection.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
        [],
        |row| row.get(0),
    )?;
    if version < 2 {
        return Err(AppError::Validation(
            "O backup pertence a uma versão incompatível".into(),
        ));
    }
    Ok(())
}

type MovementResult = (
    Option<String>,
    Option<String>,
    Option<String>,
    i64,
    serde_json::Value,
    serde_json::Value,
);

fn apply_quantity_movement(
    transaction: &Transaction<'_>,
    input: &CreateMovementInput,
    now: &str,
) -> AppResult<MovementResult> {
    let quantity = input.quantity;
    match input.kind.as_str() {
        "entry" => {
            let to = required_value(&input.to_location_id, "Informe a unidade de destino")?;
            let before = get_balance(transaction, &input.product_id, to)?;
            set_balance(transaction, &input.product_id, to, before + quantity, now)?;
            Ok((
                None,
                None,
                Some(to.to_owned()),
                quantity,
                serde_json::json!({"quantity": before}),
                serde_json::json!({"quantity": before + quantity}),
            ))
        }
        "exit" => {
            let from = required_value(&input.from_location_id, "Informe a unidade de origem")?;
            let before = get_balance(transaction, &input.product_id, from)?;
            let available = get_available_balance(transaction, &input.product_id, from)?;
            if available < quantity {
                return Err(AppError::Validation(format!(
                    "Saldo livre insuficiente: disponível {available}, solicitado {quantity}"
                )));
            }
            set_balance(transaction, &input.product_id, from, before - quantity, now)?;
            Ok((
                None,
                Some(from.to_owned()),
                None,
                quantity,
                serde_json::json!({"quantity": before}),
                serde_json::json!({"quantity": before - quantity}),
            ))
        }
        "transfer" => {
            let from = required_value(&input.from_location_id, "Informe a unidade de origem")?;
            let to = required_value(&input.to_location_id, "Informe a unidade de destino")?;
            if from == to {
                return Err(AppError::Validation(
                    "Origem e destino devem ser diferentes".into(),
                ));
            }
            let from_before = get_balance(transaction, &input.product_id, from)?;
            let available = get_available_balance(transaction, &input.product_id, from)?;
            if available < quantity {
                return Err(AppError::Validation(format!(
                    "Saldo livre insuficiente: disponível {available}, solicitado {quantity}"
                )));
            }
            let to_before = get_balance(transaction, &input.product_id, to)?;
            set_balance(
                transaction,
                &input.product_id,
                from,
                from_before - quantity,
                now,
            )?;
            set_balance(
                transaction,
                &input.product_id,
                to,
                to_before + quantity,
                now,
            )?;
            Ok((
                None,
                Some(from.to_owned()),
                Some(to.to_owned()),
                quantity,
                serde_json::json!({"origin": from_before, "destination": to_before}),
                serde_json::json!({"origin": from_before - quantity, "destination": to_before + quantity}),
            ))
        }
        "adjustment" => {
            let location = input
                .to_location_id
                .as_ref()
                .or(input.from_location_id.as_ref())
                .ok_or_else(|| AppError::Validation("Informe a unidade do ajuste".into()))?;
            let before = get_balance(transaction, &input.product_id, location)?;
            let reserved =
                before - get_available_balance(transaction, &input.product_id, location)?;
            if quantity < reserved {
                return Err(AppError::Conflict(format!(
                    "O novo saldo não pode ser menor que as {reserved} unidades reservadas"
                )));
            }
            if before == quantity {
                return Err(AppError::Validation(
                    "O novo saldo é igual ao saldo atual".into(),
                ));
            }
            set_balance(transaction, &input.product_id, location, quantity, now)?;
            let difference = (quantity - before).abs();
            let (from, to) = if quantity < before {
                (Some(location.clone()), None)
            } else {
                (None, Some(location.clone()))
            };
            Ok((
                None,
                from,
                to,
                difference,
                serde_json::json!({"quantity": before}),
                serde_json::json!({"quantity": quantity}),
            ))
        }
        _ => Err(AppError::Validation("Tipo de movimentação inválido".into())),
    }
}

fn apply_asset_movement(
    transaction: &Transaction<'_>,
    input: &CreateMovementInput,
    now: &str,
) -> AppResult<MovementResult> {
    match input.kind.as_str() {
        "entry" => {
            let to = required_value(&input.to_location_id, "Informe a unidade de destino")?;
            ensure_location(transaction, to)?;
            let asset_tag = required_text(&input.asset_tag, "Informe o patrimônio do ativo")?;
            let asset_id = Uuid::new_v4().to_string();
            transaction.execute(
                "INSERT INTO assets(id, product_id, asset_tag, serial_number, location_id, status, created_at, updated_at) VALUES (?1, ?2, ?3, NULLIF(?4, ''), ?5, 'available', ?6, ?6)",
                params![asset_id, input.product_id, asset_tag, input.serial_number.as_deref().unwrap_or("").trim(), to, now],
            )?;
            Ok((
                Some(asset_id),
                None,
                Some(to.to_owned()),
                1,
                serde_json::json!(null),
                serde_json::json!({"assetTag": asset_tag, "status": "available"}),
            ))
        }
        "exit" | "transfer" | "adjustment" => {
            let asset_id = required_value(&input.asset_id, "Selecione o ativo")?;
            let (location_id, status, in_transit): (String, String, Option<String>) = transaction
                .query_row(
                    "SELECT location_id, status, in_transit_shipment_id FROM assets WHERE id = ?1 AND product_id = ?2",
                    params![asset_id, input.product_id],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )
                .optional()?
                .ok_or_else(|| AppError::NotFound(asset_id.to_owned()))?;
            if status == "disposed" {
                return Err(AppError::Validation("Este ativo já foi baixado".into()));
            }
            let reserved: bool = transaction.query_row(
                "SELECT EXISTS(SELECT 1 FROM replenishment_transfer_asset_reservations WHERE asset_id = ?1)",
                [asset_id],
                |row| row.get(0),
            )?;
            if in_transit.is_some() || reserved {
                return Err(AppError::Conflict(
                    "O ativo está reservado ou em trânsito por uma reposição".into(),
                ));
            }
            if input.kind == "transfer" {
                let to = required_value(&input.to_location_id, "Informe a unidade de destino")?;
                if location_id == to {
                    return Err(AppError::Validation("O ativo já está nessa unidade".into()));
                }
                ensure_location(transaction, to)?;
                transaction.execute(
                    "UPDATE assets SET location_id = ?2, version = version + 1, updated_at = ?3 WHERE id = ?1",
                    params![asset_id, to, now],
                )?;
                return Ok((
                    Some(asset_id.to_owned()),
                    Some(location_id.clone()),
                    Some(to.to_owned()),
                    1,
                    serde_json::json!({"locationId": location_id}),
                    serde_json::json!({"locationId": to}),
                ));
            }
            if input.kind == "exit" {
                transaction.execute(
                    "UPDATE assets SET status = 'disposed', version = version + 1, updated_at = ?2 WHERE id = ?1",
                    params![asset_id, now],
                )?;
                return Ok((
                    Some(asset_id.to_owned()),
                    Some(location_id),
                    None,
                    1,
                    serde_json::json!({"status": status}),
                    serde_json::json!({"status": "disposed"}),
                ));
            }
            let next_status =
                required_value(&input.asset_status, "Informe o novo estado do ativo")?;
            if !matches!(
                next_status,
                "available" | "in_use" | "maintenance" | "disposed"
            ) {
                return Err(AppError::Validation("Estado do ativo inválido".into()));
            }
            if status == next_status {
                return Err(AppError::Validation("O ativo já possui esse estado".into()));
            }
            transaction.execute(
                "UPDATE assets SET status = ?2, version = version + 1, updated_at = ?3 WHERE id = ?1",
                params![asset_id, next_status, now],
            )?;
            Ok((
                Some(asset_id.to_owned()),
                Some(location_id),
                None,
                1,
                serde_json::json!({"status": status}),
                serde_json::json!({"status": next_status}),
            ))
        }
        _ => Err(AppError::Validation("Tipo de movimentação inválido".into())),
    }
}

fn master_data_table(kind: MasterDataKind) -> &'static str {
    match kind {
        MasterDataKind::Location => "locations",
        MasterDataKind::Category => "categories",
    }
}

fn master_data_entity(kind: MasterDataKind) -> &'static str {
    match kind {
        MasterDataKind::Location => "location",
        MasterDataKind::Category => "category",
    }
}

fn query_master_data(
    connection: &Connection,
    kind: MasterDataKind,
) -> AppResult<Vec<MasterDataRecord>> {
    let sql = match kind {
        MasterDataKind::Location => {
            "SELECT l.id, l.name, l.code, l.active,
               COALESCE((SELECT SUM(sb.quantity) FROM stock_balances sb WHERE sb.location_id = l.id), 0) +
               COALESCE((SELECT COUNT(*) FROM assets a WHERE a.location_id = l.id AND a.status <> 'disposed'), 0),
               l.updated_at
             FROM locations l ORDER BY l.active DESC, l.name COLLATE NOCASE"
        }
        MasterDataKind::Category => {
            "SELECT c.id, c.name, c.code, c.active,
               COALESCE((SELECT COUNT(*) FROM products p WHERE p.category_id = c.id), 0),
               c.updated_at
             FROM categories c ORDER BY c.active DESC, c.name COLLATE NOCASE"
        }
    };
    let mut statement = connection.prepare(sql)?;
    let records = statement
        .query_map([], |row| {
            Ok(MasterDataRecord {
                id: row.get(0)?,
                name: row.get(1)?,
                code: row.get(2)?,
                active: row.get(3)?,
                usage_count: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(records)
}

fn query_master_data_record(
    connection: &Connection,
    kind: MasterDataKind,
    id: &str,
) -> AppResult<Option<MasterDataRecord>> {
    Ok(query_master_data(connection, kind)?
        .into_iter()
        .find(|record| record.id == id))
}

fn validate_master_data(input: &SaveMasterDataInput) -> AppResult<()> {
    let name = input.name.trim();
    let code = normalize_code(&input.code);
    if !(2..=80).contains(&name.chars().count()) {
        return Err(AppError::Validation(
            "O nome deve ter entre 2 e 80 caracteres".into(),
        ));
    }
    if !(2..=32).contains(&code.len())
        || !code
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-'))
    {
        return Err(AppError::Validation(
            "O código deve ter de 2 a 32 letras, números, hífens ou sublinhados".into(),
        ));
    }
    Ok(())
}

fn normalize_code(value: &str) -> String {
    value.trim().to_uppercase().replace(' ', "_")
}

fn validate_item(input: &CreateItemInput) -> AppResult<()> {
    if input.sku.trim().len() < 2 || input.name.trim().len() < 2 || input.category.trim().len() < 2
    {
        return Err(AppError::Validation(
            "Código, nome e categoria devem ter ao menos 2 caracteres".into(),
        ));
    }
    if !matches!(input.tracking_type.as_str(), "quantity" | "serialized") {
        return Err(AppError::Validation("Modelo de controle inválido".into()));
    }
    if !matches!(
        input.serial_number_policy.as_str(),
        "required" | "optional" | "not_applicable"
    ) || (input.tracking_type == "quantity" && input.serial_number_policy != "not_applicable")
        || (input.tracking_type == "serialized" && input.serial_number_policy == "not_applicable")
    {
        return Err(AppError::Validation(
            "Política de número de série inválida".into(),
        ));
    }
    if input.initial_quantity != 0
        || input.location_id.is_some()
        || input.asset_tag.is_some()
        || input.serial_number.is_some()
    {
        return Err(AppError::Validation(
            "O cadastro cria somente o produto; registre o saldo por uma movimentação de entrada"
                .into(),
        ));
    }
    if input.minimum_quantity < 0 || input.initial_quantity < 0 {
        return Err(AppError::Validation(
            "Quantidades não podem ser negativas".into(),
        ));
    }
    if input.tracking_type == "serialized" && input.initial_quantity > 1 {
        return Err(AppError::Validation(
            "Cadastre um ativo patrimonial por vez".into(),
        ));
    }
    Ok(())
}

fn validate_movement(input: &CreateMovementInput) -> AppResult<()> {
    if !matches!(
        input.kind.as_str(),
        "entry" | "exit" | "transfer" | "adjustment"
    ) {
        return Err(AppError::Validation("Tipo de movimentação inválido".into()));
    }
    if input.kind == "adjustment" {
        if input.quantity < 0 {
            return Err(AppError::Validation(
                "O saldo ajustado não pode ser negativo".into(),
            ));
        }
    } else if input.quantity <= 0 {
        return Err(AppError::Validation(
            "A quantidade deve ser maior que zero".into(),
        ));
    }
    Ok(())
}

fn ensure_actor_location(
    transaction: &Transaction<'_>,
    actor: &AuthenticatedUser,
    location_id: &str,
) -> AppResult<()> {
    ensure_location(transaction, location_id)?;
    if actor.role == UserRole::Admin {
        return Ok(());
    }
    let allowed: bool = transaction.query_row(
        "SELECT EXISTS(
            SELECT 1 FROM user_locations WHERE user_id = ?1 AND location_id = ?2
        )",
        params![actor.id, location_id],
        |row| row.get(0),
    )?;
    if !allowed {
        return Err(AppError::Forbidden(
            "Seu perfil não possui acesso a esta unidade".into(),
        ));
    }
    Ok(())
}

fn ensure_actor_location_connection(
    connection: &Connection,
    actor: &AuthenticatedUser,
    location_id: &str,
) -> AppResult<()> {
    if actor.role == UserRole::Admin {
        return Ok(());
    }
    let allowed: bool = connection.query_row(
        "SELECT EXISTS(
            SELECT 1 FROM user_locations ul
            JOIN locations l ON l.id = ul.location_id
            WHERE ul.user_id = ?1 AND ul.location_id = ?2 AND l.active = 1
        )",
        params![actor.id, location_id],
        |row| row.get(0),
    )?;
    if !allowed {
        return Err(AppError::Forbidden(
            "Seu perfil não possui acesso a esta unidade".into(),
        ));
    }
    Ok(())
}

fn get_product_stock_at_location(
    transaction: &Transaction<'_>,
    product_id: &str,
    location_id: &str,
) -> AppResult<i64> {
    let tracking_type: String = transaction
        .query_row(
            "SELECT tracking_type FROM products WHERE id = ?1",
            [product_id],
            |row| row.get(0),
        )
        .optional()?
        .ok_or_else(|| AppError::NotFound(product_id.to_owned()))?;
    if tracking_type == "serialized" {
        Ok(transaction.query_row(
            "SELECT COUNT(*) FROM assets
             WHERE product_id = ?1 AND location_id = ?2 AND status <> 'disposed'",
            params![product_id, location_id],
            |row| row.get(0),
        )?)
    } else {
        get_balance(transaction, product_id, location_id)
    }
}

fn validate_replenishment_request(input: &CreateReplenishmentRequestInput) -> AppResult<()> {
    if !matches!(
        input.priority.as_str(),
        "low" | "normal" | "high" | "urgent"
    ) {
        return Err(AppError::Validation("Prioridade inválida".into()));
    }
    if !(10..=1000).contains(&input.justification.trim().chars().count()) {
        return Err(AppError::Validation(
            "A justificativa deve ter entre 10 e 1000 caracteres".into(),
        ));
    }
    if input.items.is_empty() || input.items.len() > 100 {
        return Err(AppError::Validation(
            "A solicitação deve conter de 1 a 100 produtos".into(),
        ));
    }
    let mut products = HashSet::new();
    for item in &input.items {
        if item.quantity <= 0 || item.quantity > 1_000_000 {
            return Err(AppError::Validation(
                "A quantidade solicitada deve estar entre 1 e 1.000.000".into(),
            ));
        }
        if !products.insert(item.product_id.as_str()) {
            return Err(AppError::Validation(
                "O mesmo produto não pode aparecer duas vezes na solicitação".into(),
            ));
        }
    }
    Ok(())
}

fn validate_replenishment_review(input: &ReviewReplenishmentRequestInput) -> AppResult<()> {
    if input.version < 1 || input.items.is_empty() {
        return Err(AppError::Validation("Decisão de reposição inválida".into()));
    }
    if !(10..=1000).contains(&input.note.trim().chars().count()) {
        return Err(AppError::Validation(
            "O parecer deve ter entre 10 e 1000 caracteres".into(),
        ));
    }
    Ok(())
}

fn validate_transfer_dispatch(input: &DispatchReplenishmentTransferInput) -> AppResult<()> {
    if input.items.is_empty() || input.items.len() > 200 {
        return Err(AppError::Validation(
            "O despacho deve conter entre 1 e 200 itens".into(),
        ));
    }
    if input.reference.trim().is_empty() || input.reference.chars().count() > 120 {
        return Err(AppError::Validation(
            "Informe uma referência de despacho com até 120 caracteres".into(),
        ));
    }
    if input.note.chars().count() > 1000 {
        return Err(AppError::Validation(
            "A observação do despacho deve ter até 1000 caracteres".into(),
        ));
    }
    Ok(())
}

fn validate_transfer_receipt(input: &ReceiveReplenishmentTransferInput) -> AppResult<()> {
    if input.items.is_empty() || input.items.len() > 400 {
        return Err(AppError::Validation(
            "O recebimento deve conter entre 1 e 400 linhas".into(),
        ));
    }
    if input.reference.trim().is_empty() || input.reference.chars().count() > 120 {
        return Err(AppError::Validation(
            "Informe uma referência de recebimento com até 120 caracteres".into(),
        ));
    }
    if input.note.chars().count() > 1000 {
        return Err(AppError::Validation(
            "A observação do recebimento deve ter até 1000 caracteres".into(),
        ));
    }
    Ok(())
}

fn refresh_replenishment_progress(
    transaction: &Transaction<'_>,
    request_id: &str,
    now: &str,
) -> AppResult<()> {
    transaction.execute(
        "UPDATE replenishment_request_items
         SET status = CASE
           WHEN approved_quantity = 0 THEN 'rejected'
           WHEN received_quantity = purchase_quantity
             AND transfer_received_quantity = transfer_quantity THEN 'fulfilled'
           WHEN received_quantity > 0 OR transfer_received_quantity > 0
             OR transfer_in_transit_quantity > 0 OR transfer_rejected_quantity > 0 THEN 'in_fulfillment'
           WHEN approved_quantity = requested_quantity THEN 'approved'
           ELSE 'partially_approved'
         END
         WHERE request_id = ?1",
        [request_id],
    )?;
    let (requested, approved, unfinished, progress): (i64, i64, i64, i64) = transaction.query_row(
        "SELECT COALESCE(SUM(requested_quantity), 0),
                COALESCE(SUM(approved_quantity), 0),
                SUM(CASE WHEN approved_quantity > 0 AND status <> 'fulfilled' THEN 1 ELSE 0 END),
                SUM(CASE WHEN received_quantity > 0 OR transfer_received_quantity > 0
                          OR transfer_in_transit_quantity > 0 OR transfer_rejected_quantity > 0
                         THEN 1 ELSE 0 END)
         FROM replenishment_request_items WHERE request_id = ?1",
        [request_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    )?;
    let status = if approved == 0 {
        "rejected"
    } else if unfinished == 0 {
        "fulfilled"
    } else if progress > 0 {
        "in_fulfillment"
    } else if approved == requested {
        "approved"
    } else {
        "partially_approved"
    };
    transaction.execute(
        "UPDATE replenishment_requests
         SET status = ?2, fulfilled_at = CASE WHEN ?2 = 'fulfilled' THEN ?3 ELSE NULL END,
             version = version + 1 WHERE id = ?1",
        params![request_id, status, now],
    )?;
    Ok(())
}

fn get_replenishment_request_with_connection(
    connection: &Connection,
    id: &str,
) -> AppResult<ReplenishmentRequestView> {
    let mut request = connection
        .query_row(
            "SELECT
                rr.id, rr.destination_location_id, destination.name, rr.priority,
                rr.justification, rr.status, rr.requester_id, requester.display_name,
                rr.reviewer_id, reviewer.display_name, rr.review_note, rr.requested_at,
                rr.reviewed_at, rr.fulfilled_at, rr.version
             FROM replenishment_requests rr
             JOIN locations destination ON destination.id = rr.destination_location_id
             JOIN users requester ON requester.id = rr.requester_id
             LEFT JOIN users reviewer ON reviewer.id = rr.reviewer_id
             WHERE rr.id = ?1",
            [id],
            |row| {
                Ok(ReplenishmentRequestView {
                    id: row.get(0)?,
                    destination_location_id: row.get(1)?,
                    destination_location: row.get(2)?,
                    priority: row.get(3)?,
                    justification: row.get(4)?,
                    status: row.get(5)?,
                    requester_id: row.get(6)?,
                    requester: row.get(7)?,
                    reviewer_id: row.get(8)?,
                    reviewer: row.get(9)?,
                    review_note: row.get(10)?,
                    requested_at: row.get(11)?,
                    reviewed_at: row.get(12)?,
                    fulfilled_at: row.get(13)?,
                    version: row.get(14)?,
                    items: Vec::new(),
                    transfer_shipments: Vec::new(),
                })
            },
        )
        .optional()?
        .ok_or_else(|| AppError::NotFound(id.to_owned()))?;
    let mut statement = connection.prepare(
        "SELECT
            rri.id, rri.product_id, p.name, p.sku, p.tracking_type,
            rri.requested_quantity, rri.stock_snapshot, rri.approved_quantity,
            rri.transfer_quantity, rri.purchase_quantity, rri.source_location_id,
            source.name, rri.purchase_reference, rri.status, rri.version,
            rri.received_quantity, p.serial_number_policy,
            rri.transfer_received_quantity, rri.transfer_in_transit_quantity,
            rri.transfer_rejected_quantity
         FROM replenishment_request_items rri
         JOIN products p ON p.id = rri.product_id
         LEFT JOIN locations source ON source.id = rri.source_location_id
         WHERE rri.request_id = ?1
         ORDER BY p.name COLLATE NOCASE",
    )?;
    request.items = statement
        .query_map([id], |row| {
            Ok(ReplenishmentRequestItemView {
                id: row.get(0)?,
                product_id: row.get(1)?,
                product_name: row.get(2)?,
                sku: row.get(3)?,
                tracking_type: row.get(4)?,
                serial_number_policy: row.get(16)?,
                requested_quantity: row.get(5)?,
                stock_snapshot: row.get(6)?,
                approved_quantity: row.get(7)?,
                transfer_quantity: row.get(8)?,
                purchase_quantity: row.get(9)?,
                received_quantity: row.get(15)?,
                transfer_received_quantity: row.get(17)?,
                transfer_in_transit_quantity: row.get(18)?,
                transfer_rejected_quantity: row.get(19)?,
                source_location_id: row.get(10)?,
                source_location: row.get(11)?,
                purchase_reference: row.get(12)?,
                status: row.get(13)?,
                version: row.get(14)?,
                reserved_assets: Vec::new(),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    for item in &mut request.items {
        let mut reservations = connection.prepare(
            "SELECT a.id, a.asset_tag, a.serial_number
             FROM replenishment_transfer_asset_reservations rtar
             JOIN assets a ON a.id = rtar.asset_id
             WHERE rtar.request_item_id = ?1
             ORDER BY a.asset_tag COLLATE NOCASE",
        )?;
        item.reserved_assets = reservations
            .query_map([&item.id], |row| {
                Ok(ReservedTransferAssetView {
                    id: row.get(0)?,
                    asset_tag: row.get(1)?,
                    serial_number: row.get(2)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
    }
    request.transfer_shipments = list_transfer_shipments_with_connection(connection, id)?;
    Ok(request)
}

fn list_transfer_shipments_with_connection(
    connection: &Connection,
    request_id: &str,
) -> AppResult<Vec<ReplenishmentTransferShipmentView>> {
    let mut statement = connection.prepare(
        "SELECT rts.id, rts.request_id, rts.source_location_id, source.name,
                rts.destination_location_id, destination.name, rts.reference,
                rts.dispatch_note, rts.receipt_reference, rts.receipt_note, rts.status,
                dispatcher.display_name, receiver.display_name, rts.dispatched_at,
                rts.received_at, rts.version
         FROM replenishment_transfer_shipments rts
         JOIN locations source ON source.id = rts.source_location_id
         JOIN locations destination ON destination.id = rts.destination_location_id
         JOIN users dispatcher ON dispatcher.id = rts.dispatched_by
         LEFT JOIN users receiver ON receiver.id = rts.received_by
         WHERE rts.request_id = ?1
         ORDER BY rts.dispatched_at DESC",
    )?;
    let mut shipments = statement
        .query_map([request_id], |row| {
            Ok(ReplenishmentTransferShipmentView {
                id: row.get(0)?,
                request_id: row.get(1)?,
                source_location_id: row.get(2)?,
                source_location: row.get(3)?,
                destination_location_id: row.get(4)?,
                destination_location: row.get(5)?,
                reference: row.get(6)?,
                dispatch_note: row.get(7)?,
                receipt_reference: row.get(8)?,
                receipt_note: row.get(9)?,
                status: row.get(10)?,
                dispatcher: row.get(11)?,
                receiver: row.get(12)?,
                dispatched_at: row.get(13)?,
                received_at: row.get(14)?,
                version: row.get(15)?,
                items: Vec::new(),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    drop(statement);
    for shipment in &mut shipments {
        let mut items = connection.prepare(
            "SELECT rtsi.id, rtsi.request_item_id, rtsi.product_id, p.name, p.sku,
                    p.tracking_type, rtsi.asset_id, a.asset_tag, a.serial_number,
                    rtsi.quantity, rtsi.received_quantity, rtsi.rejected_quantity,
                    rtsi.receipt_note
             FROM replenishment_transfer_shipment_items rtsi
             JOIN products p ON p.id = rtsi.product_id
             LEFT JOIN assets a ON a.id = rtsi.asset_id
             WHERE rtsi.shipment_id = ?1
             ORDER BY p.name COLLATE NOCASE, a.asset_tag COLLATE NOCASE",
        )?;
        shipment.items = items
            .query_map([&shipment.id], |row| {
                Ok(ReplenishmentTransferShipmentItemView {
                    id: row.get(0)?,
                    request_item_id: row.get(1)?,
                    product_id: row.get(2)?,
                    product_name: row.get(3)?,
                    sku: row.get(4)?,
                    tracking_type: row.get(5)?,
                    asset_id: row.get(6)?,
                    asset_tag: row.get(7)?,
                    serial_number: row.get(8)?,
                    quantity: row.get(9)?,
                    received_quantity: row.get(10)?,
                    rejected_quantity: row.get(11)?,
                    receipt_note: row.get(12)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
    }
    Ok(shipments)
}

fn validate_adjustment_request(input: &CreateAdjustmentRequestInput) -> AppResult<()> {
    let description_length = input.description.trim().chars().count();
    if !(10..=1_000).contains(&description_length) {
        return Err(AppError::Validation(
            "A justificativa deve ter entre 10 e 1.000 caracteres".into(),
        ));
    }
    match input.kind.as_str() {
        "quantity_adjustment" => {
            if input
                .requested_quantity
                .is_some_and(|quantity| quantity < 0)
            {
                return Err(AppError::Validation(
                    "A quantidade solicitada não pode ser negativa".into(),
                ));
            }
        }
        "asset_update" => {
            if input.requested_status.is_none() && input.requested_location_id.is_none() {
                return Err(AppError::Validation(
                    "Informe o novo estado ou a nova unidade do ativo".into(),
                ));
            }
            if input.requested_status.as_deref().is_some_and(|status| {
                !matches!(status, "available" | "in_use" | "maintenance" | "disposed")
            }) {
                return Err(AppError::Validation("Estado de ativo inválido".into()));
            }
        }
        _ => {
            return Err(AppError::Validation(
                "Tipo de solicitação de ajuste inválido".into(),
            ))
        }
    }
    Ok(())
}

fn validate_adjustment_review(input: &ReviewAdjustmentRequestInput) -> AppResult<()> {
    if !matches!(input.decision.as_str(), "approved" | "rejected") {
        return Err(AppError::Validation("Decisão inválida".into()));
    }
    if !(5..=1_000).contains(&input.note.trim().chars().count()) {
        return Err(AppError::Validation(
            "O parecer deve ter entre 5 e 1.000 caracteres".into(),
        ));
    }
    if input.version < 1 {
        return Err(AppError::Validation(
            "Versão da solicitação inválida".into(),
        ));
    }
    Ok(())
}

fn required_value<'a>(value: &'a Option<String>, message: &str) -> AppResult<&'a str> {
    value
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| AppError::Validation(message.into()))
}

fn required_text<'a>(value: &'a Option<String>, message: &str) -> AppResult<&'a str> {
    required_value(value, message).map(str::trim)
}

fn ensure_location(transaction: &Transaction<'_>, location_id: &str) -> AppResult<()> {
    let exists = transaction.query_row(
        "SELECT EXISTS(SELECT 1 FROM locations WHERE id = ?1 AND active = 1)",
        [location_id],
        |row| row.get::<_, bool>(0),
    )?;
    if !exists {
        return Err(AppError::NotFound(location_id.to_owned()));
    }
    Ok(())
}

fn get_balance(
    transaction: &Transaction<'_>,
    product_id: &str,
    location_id: &str,
) -> AppResult<i64> {
    ensure_location(transaction, location_id)?;
    Ok(transaction
        .query_row(
            "SELECT quantity FROM stock_balances WHERE product_id = ?1 AND location_id = ?2",
            params![product_id, location_id],
            |row| row.get(0),
        )
        .optional()?
        .unwrap_or(0))
}

fn get_available_balance(
    transaction: &Transaction<'_>,
    product_id: &str,
    location_id: &str,
) -> AppResult<i64> {
    ensure_location(transaction, location_id)?;
    Ok(transaction
        .query_row(
            "SELECT quantity - reserved_quantity FROM stock_balances WHERE product_id = ?1 AND location_id = ?2",
            params![product_id, location_id],
            |row| row.get(0),
        )
        .optional()?
        .unwrap_or(0))
}

fn set_balance(
    transaction: &Transaction<'_>,
    product_id: &str,
    location_id: &str,
    quantity: i64,
    now: &str,
) -> AppResult<()> {
    ensure_location(transaction, location_id)?;
    transaction.execute(
        "INSERT INTO stock_balances(product_id, location_id, quantity, updated_at) VALUES (?1, ?2, ?3, ?4) ON CONFLICT(product_id, location_id) DO UPDATE SET quantity = excluded.quantity, updated_at = excluded.updated_at",
        params![product_id, location_id, quantity, now],
    )?;
    Ok(())
}

fn find_or_create_category(transaction: &Transaction<'_>, name: &str) -> AppResult<String> {
    if let Some((id, active)) = transaction
        .query_row(
            "SELECT id, active FROM categories WHERE name = ?1 COLLATE NOCASE",
            [name.trim()],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, bool>(1)?)),
        )
        .optional()?
    {
        if !active {
            transaction.execute(
                "UPDATE categories SET active = 1, updated_at = ?2 WHERE id = ?1",
                params![id, Utc::now().to_rfc3339()],
            )?;
        }
        return Ok(id);
    }
    let id = Uuid::new_v4().to_string();
    let code = format!(
        "{}-{}",
        name.trim().to_uppercase().replace(' ', "_"),
        &id[..8]
    );
    let now = Utc::now().to_rfc3339();
    transaction.execute(
        "INSERT INTO categories(id, name, code, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?4)",
        params![id, name.trim(), code, now],
    )?;
    Ok(id)
}

struct AdjustmentRequestRecord {
    requester_id: String,
    product_id: String,
    asset_id: Option<String>,
    kind: String,
    location_id: Option<String>,
    requested_quantity: Option<i64>,
    requested_status: Option<String>,
    requested_location_id: Option<String>,
    status: String,
    target_version: i64,
    version: i64,
}

fn apply_approved_adjustment(
    transaction: &Transaction<'_>,
    request: &AdjustmentRequestRecord,
    reviewer_id: &str,
    request_id: &str,
    review_note: &str,
    now: &str,
) -> AppResult<()> {
    let movement_note = format!(
        "Ajuste aprovado na solicitação {request_id}: {}",
        review_note.trim()
    );
    if request.kind == "quantity_adjustment" {
        let location_id = request
            .location_id
            .as_deref()
            .ok_or_else(|| AppError::Security("Solicitação de quantidade sem unidade".into()))?;
        let requested_quantity = request
            .requested_quantity
            .ok_or_else(|| AppError::Security("Solicitação de quantidade sem valor".into()))?;
        let product_version: i64 = transaction
            .query_row(
                "SELECT version FROM products WHERE id = ?1 AND active = 1",
                [&request.product_id],
                |row| row.get(0),
            )
            .optional()?
            .ok_or_else(|| AppError::NotFound(request.product_id.clone()))?;
        if product_version != request.target_version {
            return Err(AppError::Conflict(
                "O estoque mudou depois da solicitação. Recuse-a e solicite uma nova análise"
                    .into(),
            ));
        }
        let current_quantity = get_balance(transaction, &request.product_id, location_id)?;
        if current_quantity == requested_quantity {
            return Err(AppError::Conflict(
                "O estoque já possui a quantidade solicitada".into(),
            ));
        }
        let movement_quantity = current_quantity.abs_diff(requested_quantity);
        let movement_quantity = i64::try_from(movement_quantity).map_err(|_| {
            AppError::Validation("A diferença de quantidade ultrapassa o limite aceito".into())
        })?;
        set_balance(
            transaction,
            &request.product_id,
            location_id,
            requested_quantity,
            now,
        )?;
        let (from_location, to_location) = if requested_quantity < current_quantity {
            (Some(location_id), None)
        } else {
            (None, Some(location_id))
        };
        insert_movement(
            transaction,
            &request.product_id,
            None,
            "adjustment",
            movement_quantity,
            from_location,
            to_location,
            &movement_note,
            reviewer_id,
            now,
        )?;
        let updated = transaction.execute(
            "UPDATE products SET version = version + 1, updated_at = ?2 WHERE id = ?1 AND version = ?3",
            params![request.product_id, now, request.target_version],
        )?;
        if updated != 1 {
            return Err(AppError::Conflict(
                "O produto foi alterado durante a aprovação".into(),
            ));
        }
        return Ok(());
    }

    let asset_id = request
        .asset_id
        .as_deref()
        .ok_or_else(|| AppError::Security("Solicitação patrimonial sem ativo".into()))?;
    let (current_location, current_status, current_version): (String, String, i64) = transaction
        .query_row(
            "SELECT location_id, status, version FROM assets WHERE id = ?1",
            [asset_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?
        .ok_or_else(|| AppError::NotFound(asset_id.to_owned()))?;
    if current_version != request.target_version {
        return Err(AppError::Conflict(
            "O ativo mudou depois da solicitação. Recuse-a e solicite uma nova análise".into(),
        ));
    }
    let target_location = request
        .requested_location_id
        .as_deref()
        .unwrap_or(&current_location);
    let target_status = request
        .requested_status
        .as_deref()
        .unwrap_or(&current_status);
    ensure_location(transaction, target_location)?;
    if target_location == current_location && target_status == current_status {
        return Err(AppError::Conflict(
            "O ativo já possui o estado solicitado".into(),
        ));
    }
    let updated = transaction.execute(
        "UPDATE assets SET location_id = ?2, status = ?3, version = version + 1, updated_at = ?4 WHERE id = ?1 AND version = ?5",
        params![asset_id, target_location, target_status, now, request.target_version],
    )?;
    if updated != 1 {
        return Err(AppError::Conflict(
            "O ativo foi alterado durante a aprovação".into(),
        ));
    }
    if target_location != current_location {
        insert_movement(
            transaction,
            &request.product_id,
            Some(asset_id),
            "transfer",
            1,
            Some(&current_location),
            Some(target_location),
            &movement_note,
            reviewer_id,
            now,
        )?;
    }
    if target_status != current_status {
        insert_movement(
            transaction,
            &request.product_id,
            Some(asset_id),
            "adjustment",
            1,
            Some(target_location),
            None,
            &movement_note,
            reviewer_id,
            now,
        )?;
    }
    transaction.execute(
        "UPDATE products SET version = version + 1, updated_at = ?2 WHERE id = ?1",
        params![request.product_id, now],
    )?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn insert_movement(
    transaction: &Transaction<'_>,
    product_id: &str,
    asset_id: Option<&str>,
    kind: &str,
    quantity: i64,
    from_location_id: Option<&str>,
    to_location_id: Option<&str>,
    note: &str,
    actor_id: &str,
    now: &str,
) -> AppResult<String> {
    let id = Uuid::new_v4().to_string();
    transaction.execute(
        "INSERT INTO stock_movements(id, product_id, asset_id, kind, quantity, from_location_id, to_location_id, actor_id, note, occurred_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![id, product_id, asset_id, kind, quantity, from_location_id, to_location_id, actor_id, note, now],
    )?;
    Ok(id)
}

#[allow(clippy::too_many_arguments)]
fn insert_audit(
    transaction: &Transaction<'_>,
    action: &str,
    entity_type: &str,
    entity_id: &str,
    before: Option<serde_json::Value>,
    after: Option<serde_json::Value>,
    actor_id: &str,
    now: &str,
) -> AppResult<()> {
    transaction.execute(
        "INSERT INTO audit_logs(id, actor_id, action, entity_type, entity_id, before_data, after_data, occurred_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![Uuid::new_v4().to_string(), actor_id, action, entity_type, entity_id, before.map(|value| value.to_string()), after.map(|value| value.to_string()), now],
    )?;
    Ok(())
}

fn query_items(
    connection: &Connection,
    search: &str,
    low_stock_only: bool,
) -> AppResult<Vec<InventoryItem>> {
    let term = format!("%{}%", search.trim());
    let low_stock = i64::from(low_stock_only);
    let mut statement = connection.prepare(
        "SELECT p.id, p.sku, p.name, c.name, p.tracking_type, p.serial_number_policy,
           CASE WHEN p.tracking_type = 'quantity' THEN COALESCE((SELECT SUM(sb.quantity) FROM stock_balances sb WHERE sb.product_id = p.id), 0) ELSE COALESCE((SELECT COUNT(*) FROM assets a WHERE a.product_id = p.id AND a.status <> 'disposed'), 0) END,
           p.minimum_quantity,
           CASE WHEN p.tracking_type = 'quantity'
             THEN COALESCE((SELECT GROUP_CONCAT(label, ', ') FROM (SELECT l.name || ' (' || sb.quantity || ')' AS label FROM stock_balances sb JOIN locations l ON l.id = sb.location_id WHERE sb.product_id = p.id AND sb.quantity > 0 ORDER BY l.name)), 'Sem saldo')
             ELSE COALESCE((SELECT GROUP_CONCAT(label, ', ') FROM (SELECT l.name || ' (' || COUNT(*) || ')' AS label FROM assets a JOIN locations l ON l.id = a.location_id WHERE a.product_id = p.id AND a.status <> 'disposed' GROUP BY l.id, l.name ORDER BY l.name)), 'Sem ativos')
           END,
           p.active, p.updated_at
         FROM products p JOIN categories c ON c.id = p.category_id
         WHERE (?1 = '%%' OR p.sku LIKE ?1 OR p.name LIKE ?1 OR c.name LIKE ?1 OR EXISTS (SELECT 1 FROM assets a WHERE a.product_id = p.id AND (a.asset_tag LIKE ?1 OR a.serial_number LIKE ?1)))
         AND (?2 = 0 OR (p.tracking_type = 'quantity' AND COALESCE((SELECT SUM(sb.quantity) FROM stock_balances sb WHERE sb.product_id = p.id), 0) < p.minimum_quantity))
         ORDER BY p.updated_at DESC, p.name COLLATE NOCASE",
    )?;
    let items = statement
        .query_map(params![term, low_stock], map_item)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(items)
}

fn query_items_for(
    connection: &Connection,
    search: &str,
    low_stock_only: bool,
    actor: &AuthenticatedUser,
) -> AppResult<Vec<InventoryItem>> {
    if actor.role == UserRole::Admin {
        return query_items(connection, search, low_stock_only);
    }
    let term = format!("%{}%", search.trim());
    let low_stock = i64::from(low_stock_only);
    let mut statement = connection.prepare(
        "SELECT p.id, p.sku, p.name, c.name, p.tracking_type, p.serial_number_policy,
           CASE WHEN p.tracking_type = 'quantity'
             THEN COALESCE((SELECT SUM(sb.quantity) FROM stock_balances sb JOIN user_locations ul ON ul.location_id = sb.location_id WHERE sb.product_id = p.id AND ul.user_id = ?3), 0)
             ELSE COALESCE((SELECT COUNT(*) FROM assets a JOIN user_locations ul ON ul.location_id = a.location_id WHERE a.product_id = p.id AND a.status <> 'disposed' AND ul.user_id = ?3), 0)
           END,
           COALESCE((SELECT MAX(sp.minimum_quantity) FROM stock_policies sp JOIN user_locations ul ON ul.location_id = sp.location_id WHERE sp.product_id = p.id AND ul.user_id = ?3), p.minimum_quantity),
           CASE WHEN p.tracking_type = 'quantity'
             THEN COALESCE((SELECT GROUP_CONCAT(label, ', ') FROM (SELECT l.name || ' (' || sb.quantity || ')' AS label FROM stock_balances sb JOIN locations l ON l.id = sb.location_id JOIN user_locations ul ON ul.location_id = l.id WHERE sb.product_id = p.id AND sb.quantity > 0 AND ul.user_id = ?3 ORDER BY l.name)), 'Sem saldo')
             ELSE COALESCE((SELECT GROUP_CONCAT(label, ', ') FROM (SELECT l.name || ' (' || COUNT(*) || ')' AS label FROM assets a JOIN locations l ON l.id = a.location_id JOIN user_locations ul ON ul.location_id = l.id WHERE a.product_id = p.id AND a.status <> 'disposed' AND ul.user_id = ?3 GROUP BY l.id, l.name ORDER BY l.name)), 'Sem ativos')
           END,
           p.active, p.updated_at
         FROM products p JOIN categories c ON c.id = p.category_id
         WHERE (?1 = '%%' OR p.sku LIKE ?1 OR p.name LIKE ?1 OR c.name LIKE ?1 OR EXISTS (
             SELECT 1 FROM assets a JOIN user_locations ul ON ul.location_id = a.location_id
             WHERE a.product_id = p.id AND ul.user_id = ?3 AND (a.asset_tag LIKE ?1 OR a.serial_number LIKE ?1)
         ))
         AND (?2 = 0 OR (
             p.tracking_type = 'quantity' AND EXISTS (
                 SELECT 1 FROM user_locations ul
                 LEFT JOIN stock_balances sb ON sb.location_id = ul.location_id AND sb.product_id = p.id
                 LEFT JOIN stock_policies sp ON sp.location_id = ul.location_id AND sp.product_id = p.id
                 WHERE ul.user_id = ?3 AND COALESCE(sb.quantity, 0) < COALESCE(sp.minimum_quantity, p.minimum_quantity)
             )
         ))
         ORDER BY p.updated_at DESC, p.name COLLATE NOCASE",
    )?;
    let items = statement
        .query_map(params![term, low_stock, actor.id], map_item)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(items)
}

fn map_item(row: &rusqlite::Row<'_>) -> rusqlite::Result<InventoryItem> {
    Ok(InventoryItem {
        id: row.get(0)?,
        sku: row.get(1)?,
        name: row.get(2)?,
        category: row.get(3)?,
        tracking_type: row.get(4)?,
        serial_number_policy: row.get(5)?,
        quantity: row.get(6)?,
        minimum_quantity: row.get(7)?,
        locations: row.get(8)?,
        active: row.get(9)?,
        updated_at: row.get(10)?,
    })
}

fn map_asset(row: &rusqlite::Row<'_>) -> rusqlite::Result<AssetView> {
    Ok(AssetView {
        id: row.get(0)?,
        product_id: row.get(1)?,
        asset_tag: row.get(2)?,
        serial_number: row.get(3)?,
        location_id: row.get(4)?,
        location: row.get(5)?,
        status: row.get(6)?,
        receipt_batch_id: row.get(7)?,
        updated_at: row.get(8)?,
    })
}

const ASSET_RECORD_SELECT: &str =
    "SELECT a.id, a.product_id, p.sku, p.name, c.name, a.asset_tag, a.serial_number,
       a.location_id, l.name, a.status, a.receipt_batch_id, a.updated_at
     FROM assets a
     JOIN products p ON p.id = a.product_id
     JOIN categories c ON c.id = p.category_id
     JOIN locations l ON l.id = a.location_id";

fn query_asset_records(
    connection: &Connection,
    filters: &AssetFilterInput,
) -> AppResult<Vec<AssetRecordView>> {
    let search = format!("%{}%", filters.search.trim());
    let sql = format!(
        "{ASSET_RECORD_SELECT}
         WHERE (?1 = '%%' OR a.asset_tag LIKE ?1 OR COALESCE(a.serial_number, '') LIKE ?1 OR p.sku LIKE ?1 OR p.name LIKE ?1 OR c.name LIKE ?1)
           AND (?2 IS NULL OR a.location_id = ?2)
           AND (?3 IS NULL OR a.status = ?3)
         ORDER BY CASE a.status WHEN 'maintenance' THEN 0 WHEN 'available' THEN 1 WHEN 'in_use' THEN 2 ELSE 3 END,
                  a.updated_at DESC, a.asset_tag COLLATE NOCASE"
    );
    let mut statement = connection.prepare(&sql)?;
    let records = statement
        .query_map(
            params![search, filters.location_id, filters.status],
            map_asset_record,
        )?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(records)
}

fn query_asset_record_by_id(
    connection: &Connection,
    id: &str,
) -> AppResult<Option<AssetRecordView>> {
    let sql = format!("{ASSET_RECORD_SELECT} WHERE a.id = ?1");
    Ok(connection
        .query_row(&sql, [id], map_asset_record)
        .optional()?)
}

fn map_asset_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<AssetRecordView> {
    Ok(AssetRecordView {
        id: row.get(0)?,
        product_id: row.get(1)?,
        sku: row.get(2)?,
        product_name: row.get(3)?,
        category: row.get(4)?,
        asset_tag: row.get(5)?,
        serial_number: row.get(6)?,
        location_id: row.get(7)?,
        location: row.get(8)?,
        status: row.get(9)?,
        receipt_batch_id: row.get(10)?,
        updated_at: row.get(11)?,
    })
}

fn validate_asset_filters(filters: &AssetFilterInput) -> AppResult<()> {
    if filters.search.chars().count() > 160 {
        return Err(AppError::Validation(
            "A busca deve ter no máximo 160 caracteres".into(),
        ));
    }
    if let Some(status) = &filters.status {
        if !matches!(
            status.as_str(),
            "available" | "in_use" | "maintenance" | "disposed"
        ) {
            return Err(AppError::Validation("Estado do ativo inválido".into()));
        }
    }
    Ok(())
}

fn validate_asset_update(input: &UpdateAssetInput) -> AppResult<()> {
    if !matches!(
        input.status.as_str(),
        "available" | "in_use" | "maintenance"
    ) {
        return Err(AppError::Validation(
            "Use uma saída para baixar o ativo".into(),
        ));
    }
    if input.note.chars().count() > 500 {
        return Err(AppError::Validation(
            "A observação deve ter no máximo 500 caracteres".into(),
        ));
    }
    Ok(())
}

const AUDIT_SELECT: &str =
    "SELECT a.id, COALESCE(u.display_name, u.username, 'Sistema') AS actor, a.action, a.entity_type,
       a.entity_id,
       CASE a.entity_type
         WHEN 'product' THEN COALESCE((SELECT p.name FROM products p WHERE p.id = a.entity_id), a.entity_id)
         WHEN 'stock' THEN COALESCE((SELECT p.name FROM products p WHERE p.id = a.entity_id), a.entity_id)
         WHEN 'asset' THEN COALESCE((SELECT x.asset_tag FROM assets x WHERE x.id = a.entity_id), a.entity_id)
         WHEN 'location' THEN COALESCE((SELECT l.name FROM locations l WHERE l.id = a.entity_id), a.entity_id)
         WHEN 'category' THEN COALESCE((SELECT c.name FROM categories c WHERE c.id = a.entity_id), a.entity_id)
         ELSE a.entity_id
       END AS entity_name,
       a.before_data, a.after_data, a.occurred_at
     FROM audit_logs a LEFT JOIN users u ON u.id = a.actor_id";

fn query_audit_logs(
    connection: &Connection,
    filters: &AuditFilterInput,
) -> AppResult<Vec<AuditLogView>> {
    let search = format!("%{}%", filters.search.trim());
    let sql = format!(
        "SELECT * FROM ({AUDIT_SELECT}) audit
         WHERE (?1 = '%%' OR audit.actor LIKE ?1 OR audit.action LIKE ?1 OR audit.entity_type LIKE ?1 OR audit.entity_id LIKE ?1 OR audit.entity_name LIKE ?1)
           AND (?2 IS NULL OR audit.entity_type = ?2)
           AND (?3 IS NULL OR audit.action = ?3)
           AND (?4 IS NULL OR date(audit.occurred_at) >= date(?4))
           AND (?5 IS NULL OR date(audit.occurred_at) <= date(?5))
         ORDER BY audit.occurred_at DESC, audit.id DESC LIMIT 500"
    );
    let mut statement = connection.prepare(&sql)?;
    let logs = statement
        .query_map(
            params![
                search,
                filters.entity_type,
                filters.action,
                filters.date_from,
                filters.date_to
            ],
            map_audit_log,
        )?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(logs)
}

fn map_audit_log(row: &rusqlite::Row<'_>) -> rusqlite::Result<AuditLogView> {
    let before: Option<String> = row.get(6)?;
    let after: Option<String> = row.get(7)?;
    Ok(AuditLogView {
        id: row.get(0)?,
        actor: row.get(1)?,
        action: row.get(2)?,
        entity_type: row.get(3)?,
        entity_id: row.get(4)?,
        entity_name: row.get(5)?,
        before_data: before.and_then(|value| serde_json::from_str(&value).ok()),
        after_data: after.and_then(|value| serde_json::from_str(&value).ok()),
        occurred_at: row.get(8)?,
    })
}

fn validate_audit_filters(filters: &AuditFilterInput) -> AppResult<()> {
    if filters.search.chars().count() > 160 {
        return Err(AppError::Validation(
            "A busca deve ter no máximo 160 caracteres".into(),
        ));
    }
    let parse_date = |value: &Option<String>| -> AppResult<Option<NaiveDate>> {
        value
            .as_deref()
            .map(|date| {
                NaiveDate::parse_from_str(date, "%Y-%m-%d")
                    .map_err(|_| AppError::Validation("Data de auditoria inválida".into()))
            })
            .transpose()
    };
    let date_from = parse_date(&filters.date_from)?;
    let date_to = parse_date(&filters.date_to)?;
    if matches!((date_from, date_to), (Some(from), Some(to)) if from > to) {
        return Err(AppError::Validation(
            "A data inicial não pode ser posterior à data final".into(),
        ));
    }
    Ok(())
}

const MOVEMENT_SELECT: &str =
    "SELECT m.id, m.occurred_at, m.kind, p.name, a.asset_tag,
       CASE WHEN m.kind = 'transfer' THEN COALESCE(fl.name, '') || ' → ' || COALESCE(tl.name, '') ELSE COALESCE(tl.name, fl.name, '') END,
       c.name, m.quantity, COALESCE(u.username, 'sistema'), m.note
     FROM stock_movements m
     JOIN products p ON p.id = m.product_id
     JOIN categories c ON c.id = p.category_id
     LEFT JOIN assets a ON a.id = m.asset_id
     LEFT JOIN locations fl ON fl.id = m.from_location_id
     LEFT JOIN locations tl ON tl.id = m.to_location_id
     LEFT JOIN users u ON u.id = m.actor_id";

const ASSET_INCIDENT_SELECT: &str =
    "SELECT ai.id, ai.asset_id, a.asset_tag, a.serial_number, p.name, l.name,
            a.receipt_batch_id, reporter.display_name, ai.custodian_name, ai.description,
            ai.status, reviewer.display_name, ai.resolution_note, ai.reported_at,
            ai.reviewed_at, ai.resolved_at, ai.version
       FROM asset_incidents ai
       JOIN assets a ON a.id = ai.asset_id
       JOIN products p ON p.id = a.product_id
       JOIN locations l ON l.id = a.location_id
       JOIN users reporter ON reporter.id = ai.reporter_id
  LEFT JOIN users reviewer ON reviewer.id = ai.reviewer_id";

fn map_asset_incident(row: &rusqlite::Row<'_>) -> rusqlite::Result<AssetIncidentView> {
    Ok(AssetIncidentView {
        id: row.get(0)?,
        asset_id: row.get(1)?,
        asset_tag: row.get(2)?,
        serial_number: row.get(3)?,
        product_name: row.get(4)?,
        location: row.get(5)?,
        receipt_batch_id: row.get(6)?,
        reporter: row.get(7)?,
        custodian_name: row.get(8)?,
        description: row.get(9)?,
        status: row.get(10)?,
        reviewer: row.get(11)?,
        resolution_note: row.get(12)?,
        reported_at: row.get(13)?,
        reviewed_at: row.get(14)?,
        resolved_at: row.get(15)?,
        version: row.get(16)?,
    })
}

fn query_asset_incident(connection: &Connection, id: &str) -> AppResult<Option<AssetIncidentView>> {
    let sql = format!("{ASSET_INCIDENT_SELECT} WHERE ai.id = ?1");
    Ok(connection
        .query_row(&sql, [id], map_asset_incident)
        .optional()?)
}

const ADJUSTMENT_REQUEST_SELECT: &str =
    "SELECT ar.id, ar.kind, ar.status, ar.product_id, p.name, p.sku,
            ar.asset_id, a.asset_tag, ar.location_id, l.name,
            ar.requested_quantity, ar.requested_status, ar.requested_location_id, rl.name,
            ar.description, ar.requester_id, requester.display_name,
            ar.reviewer_id, reviewer.display_name, ar.review_note,
            ar.requested_at, ar.reviewed_at, ar.version
       FROM adjustment_requests ar
       JOIN products p ON p.id = ar.product_id
       JOIN users requester ON requester.id = ar.requester_id
  LEFT JOIN users reviewer ON reviewer.id = ar.reviewer_id
  LEFT JOIN assets a ON a.id = ar.asset_id
  LEFT JOIN locations l ON l.id = ar.location_id
  LEFT JOIN locations rl ON rl.id = ar.requested_location_id";

fn map_adjustment_request(row: &rusqlite::Row<'_>) -> rusqlite::Result<AdjustmentRequestView> {
    Ok(AdjustmentRequestView {
        id: row.get(0)?,
        kind: row.get(1)?,
        status: row.get(2)?,
        product_id: row.get(3)?,
        product_name: row.get(4)?,
        sku: row.get(5)?,
        asset_id: row.get(6)?,
        asset_tag: row.get(7)?,
        location_id: row.get(8)?,
        location: row.get(9)?,
        requested_quantity: row.get(10)?,
        requested_status: row.get(11)?,
        requested_location_id: row.get(12)?,
        requested_location: row.get(13)?,
        description: row.get(14)?,
        requester_id: row.get(15)?,
        requester: row.get(16)?,
        reviewer_id: row.get(17)?,
        reviewer: row.get(18)?,
        review_note: row.get(19)?,
        requested_at: row.get(20)?,
        reviewed_at: row.get(21)?,
        version: row.get(22)?,
    })
}

fn query_movements(connection: &Connection, limit: i64) -> AppResult<Vec<MovementView>> {
    let sql = format!("{MOVEMENT_SELECT} ORDER BY m.occurred_at DESC, m.rowid DESC LIMIT ?1");
    let mut statement = connection.prepare(&sql)?;
    let movements = statement
        .query_map([limit], map_movement)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(movements)
}

fn query_movement_by_id(connection: &Connection, id: &str) -> AppResult<Option<MovementView>> {
    let sql = format!("{MOVEMENT_SELECT} WHERE m.id = ?1");
    Ok(connection.query_row(&sql, [id], map_movement).optional()?)
}

fn map_movement(row: &rusqlite::Row<'_>) -> rusqlite::Result<MovementView> {
    Ok(MovementView {
        id: row.get(0)?,
        occurred_at: row.get(1)?,
        kind: row.get(2)?,
        item_name: row.get(3)?,
        asset_tag: row.get(4)?,
        location: row.get(5)?,
        category: row.get(6)?,
        quantity: row.get(7)?,
        actor: row.get(8)?,
        note: row.get(9)?,
    })
}

fn map_authenticated_user(row: &rusqlite::Row<'_>) -> rusqlite::Result<AuthenticatedUser> {
    let role: String = row.get(3)?;
    let role = UserRole::parse(&role).ok_or(rusqlite::Error::InvalidQuery)?;
    Ok(AuthenticatedUser {
        id: row.get(0)?,
        username: row.get(1)?,
        display_name: row.get(2)?,
        role,
    })
}

fn map_auth_user(row: &rusqlite::Row<'_>) -> rusqlite::Result<AuthUserRecord> {
    Ok(AuthUserRecord {
        user: map_authenticated_user(row)?,
        password_hash: row.get(4)?,
        failed_login_attempts: row.get(5)?,
        locked_until: row.get(6)?,
        session_version: row.get(7)?,
    })
}

fn map_user_view(row: &rusqlite::Row<'_>) -> rusqlite::Result<UserView> {
    let role: String = row.get(3)?;
    let role = UserRole::parse(&role).ok_or(rusqlite::Error::InvalidQuery)?;
    Ok(UserView {
        id: row.get(0)?,
        username: row.get(1)?,
        display_name: row.get(2)?,
        role,
        active: row.get(4)?,
        failed_login_attempts: row.get(5)?,
        locked_until: row.get(6)?,
        last_login_at: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
        location_ids: Vec::new(),
    })
}

fn query_user_location_ids(connection: &Connection, user_id: &str) -> AppResult<Vec<String>> {
    let mut statement = connection.prepare(
        "SELECT location_id FROM user_locations WHERE user_id = ?1 ORDER BY location_id",
    )?;
    let location_ids = statement
        .query_map([user_id], |row| row.get(0))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(location_ids)
}

fn sync_user_locations(
    transaction: &Transaction<'_>,
    user_id: &str,
    role: UserRole,
    location_ids: &[String],
    now: &str,
) -> AppResult<()> {
    transaction.execute("DELETE FROM user_locations WHERE user_id = ?1", [user_id])?;
    if role == UserRole::Admin {
        return Ok(());
    }

    let unique = location_ids.iter().collect::<HashSet<_>>();
    if unique.is_empty() {
        return Err(AppError::Validation(
            "Operadores e gestores devem estar vinculados a pelo menos uma unidade".into(),
        ));
    }
    for location_id in unique {
        let exists: bool = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM locations WHERE id = ?1 AND active = 1)",
            [location_id],
            |row| row.get(0),
        )?;
        if !exists {
            return Err(AppError::Validation(format!(
                "A unidade {location_id} não existe ou está inativa"
            )));
        }
        transaction.execute(
            "INSERT INTO user_locations(user_id, location_id, assigned_at) VALUES (?1, ?2, ?3)",
            params![user_id, location_id, now],
        )?;
    }
    Ok(())
}

fn seed_database(connection: &Connection) -> AppResult<()> {
    let now = "2025-05-24T10:15:00-03:00";
    connection.execute(
        "INSERT OR IGNORE INTO users(id, username, display_name, role, created_at, updated_at) VALUES (?1, 'admin', 'Administrador', 'admin', ?2, ?2)",
        params![ADMIN_ID, now],
    )?;
    let locations = [
        (
            "2d6c9f1f-dbf5-4321-8500-000000000001",
            "Escritório Central",
            "CENTRAL",
        ),
        ("2d6c9f1f-dbf5-4321-8500-000000000002", "Filial Sul", "SUL"),
        (
            "2d6c9f1f-dbf5-4321-8500-000000000003",
            "Depósito",
            "DEPOSITO",
        ),
    ];
    for (id, name, code) in locations {
        connection.execute(
            "INSERT OR IGNORE INTO locations(id, name, code, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?4)",
            params![id, name, code, now],
        )?;
    }
    let categories = [
        (
            "3d6c9f1f-dbf5-4321-8500-000000000001",
            "Computadores",
            "COMPUTADORES",
        ),
        (
            "3d6c9f1f-dbf5-4321-8500-000000000002",
            "Notebooks",
            "NOTEBOOKS",
        ),
        (
            "3d6c9f1f-dbf5-4321-8500-000000000003",
            "Monitores",
            "MONITORES",
        ),
        (
            "3d6c9f1f-dbf5-4321-8500-000000000006",
            "Periféricos",
            "PERIFERICOS",
        ),
    ];
    for (id, name, code) in categories {
        connection.execute(
            "INSERT OR IGNORE INTO categories(id, name, code, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?4)",
            params![id, name, code, now],
        )?;
    }

    let product_count: i64 =
        connection.query_row("SELECT COUNT(*) FROM products", [], |row| row.get(0))?;
    if product_count > 0 {
        return Ok(());
    }

    let products = [
        (
            "fca1cd57-e8fc-45d6-bdda-07632f951ec5",
            "NOTE-E14",
            "Notebook Lenovo ThinkPad E14",
            categories[1].0,
            "serialized",
            0,
        ),
        (
            "ea681329-cdb5-46b2-b991-d25b55c21f40",
            "MON-DELL24",
            "Monitor Dell 24\"",
            categories[2].0,
            "quantity",
            2,
        ),
        (
            "9e5f6689-c287-4980-af21-73ff042b0399",
            "MOUSE-MX3",
            "Mouse Logitech MX Master 3S",
            categories[3].0,
            "quantity",
            2,
        ),
        (
            "3e4b1121-e05a-4838-b425-910376187a24",
            "OPT-7010",
            "Desktop Dell OptiPlex 7010",
            categories[0].0,
            "serialized",
            0,
        ),
    ];
    for (id, sku, name, category, tracking, minimum) in products {
        connection.execute(
            "INSERT INTO products(id, sku, name, category_id, tracking_type, minimum_quantity, serial_number_policy, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, CASE WHEN ?5 = 'serialized' THEN 'required' ELSE 'not_applicable' END, ?7, ?7)",
            params![id, sku, name, category, tracking, minimum, now],
        )?;
    }
    connection.execute(
        "INSERT INTO stock_balances(product_id, location_id, quantity, updated_at) VALUES (?1, ?2, 1, ?4), (?3, ?2, 4, ?4)",
        params![products[1].0, locations[1].0, products[2].0, now],
    )?;
    connection.execute(
        "INSERT OR IGNORE INTO stock_policies(product_id, location_id, minimum_quantity, target_quantity, updated_at)
         SELECT sb.product_id, sb.location_id, p.minimum_quantity, MAX(p.minimum_quantity, sb.quantity), ?1
         FROM stock_balances sb JOIN products p ON p.id = sb.product_id",
        [now],
    )?;
    let assets = [
        (
            "7a6c9f1f-dbf5-4321-8500-000000000001",
            products[0].0,
            "NB-0014",
            "PF4ABC1",
            locations[0].0,
            "available",
        ),
        (
            "7a6c9f1f-dbf5-4321-8500-000000000002",
            products[0].0,
            "NB-0015",
            "PF4ABC2",
            locations[0].0,
            "in_use",
        ),
        (
            "7a6c9f1f-dbf5-4321-8500-000000000003",
            products[3].0,
            "PC-0009",
            "D7010-09",
            locations[0].0,
            "maintenance",
        ),
    ];
    for (id, product, tag, serial, location, status) in assets {
        connection.execute(
            "INSERT INTO assets(id, product_id, asset_tag, serial_number, location_id, status, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
            params![id, product, tag, serial, location, status, now],
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::imports::{StagedAsset, StagedProduct};

    fn file_database() -> (Database, PathBuf) {
        let directory = std::env::temp_dir().join(format!("stockmanager-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&directory).expect("test directory");
        let database = Database::open(directory.join("stockmanager.db")).expect("database");
        (database, directory)
    }

    fn quantity_product(database: &Database) -> InventoryItem {
        database
            .list_items("")
            .expect("list")
            .into_iter()
            .find(|item| item.tracking_type == "quantity")
            .expect("quantity product")
    }

    fn insert_test_user(database: &Database, id: &str, username: &str, role: &str) {
        let connection = database.connection.lock();
        connection
            .execute(
                "INSERT INTO users(id, username, display_name, role, active, created_at, updated_at) VALUES (?1, ?2, ?2, ?3, 1, 'now', 'now')",
                params![id, username, role],
            )
            .expect("test user");
        if role != "admin" {
            connection
                .execute(
                    "INSERT INTO user_locations(user_id, location_id, assigned_at)
                     SELECT ?1, id, 'now' FROM locations WHERE active = 1",
                    [id],
                )
                .expect("test user locations");
        }
    }

    fn assert_file_is_encrypted(path: &Path) {
        let bytes = fs::read(path).expect("read database");
        assert!(bytes.len() >= 16);
        assert_ne!(&bytes[..16], b"SQLite format 3\0");
        let without_key = Connection::open(path).expect("open without key");
        assert!(without_key
            .query_row("SELECT COUNT(*) FROM sqlite_master", [], |row| row
                .get::<_, i64>(0))
            .is_err());
    }

    #[test]
    fn encrypts_new_database_and_rejects_access_without_key() {
        let (database, directory) = file_database();
        let path = database.path().to_path_buf();
        drop(database);
        assert_file_is_encrypted(&path);
        fs::remove_dir_all(directory).expect("cleanup");
    }

    #[test]
    fn converts_existing_plaintext_database_without_losing_schema() {
        let directory =
            std::env::temp_dir().join(format!("stockmanager-legacy-{}", Uuid::new_v4()));
        fs::create_dir_all(&directory).expect("test directory");
        let path = directory.join("stockmanager.db");
        let plaintext = Connection::open(&path).expect("plaintext database");
        plaintext
            .execute_batch(MIGRATION_0001)
            .expect("legacy schema");
        drop(plaintext);

        let database = Database::open(path.clone()).expect("encrypted migration");
        assert!(database.list_items("").is_ok());
        drop(database);
        assert_file_is_encrypted(&path);
        fs::remove_dir_all(directory).expect("cleanup");
    }

    #[test]
    fn audits_recovery_key_reveal_and_acknowledgement() {
        let database = Database::open(PathBuf::from(":memory:")).expect("database");
        assert!(
            !database
                .recovery_key_status()
                .expect("initial status")
                .acknowledged
        );
        let recovery_key = database.reveal_recovery_key(ADMIN_ID).expect("reveal key");
        assert_eq!(recovery_key.len(), 64);
        assert!(
            database
                .acknowledge_recovery_key(ADMIN_ID)
                .expect("acknowledge")
                .acknowledged
        );
        let logs = database
            .list_audit_logs(AuditFilterInput {
                search: "database_encryption".into(),
                entity_type: Some("security_configuration".into()),
                action: None,
                date_from: None,
                date_to: None,
            })
            .expect("audit logs");
        assert_eq!(logs.len(), 2);
        assert!(logs.iter().all(|log| {
            !log.after_data
                .as_ref()
                .is_some_and(|value| value.to_string().contains(&recovery_key))
        }));
    }

    #[test]
    fn migrates_legacy_items_without_losing_stock() {
        let connection = Connection::open_in_memory().expect("connection");
        connection
            .execute_batch(MIGRATION_0001)
            .expect("migration 1");
        connection.execute(
            "INSERT INTO categories(id, name, code, created_at, updated_at) VALUES ('category', 'Cabos', 'CABOS', 'now', 'now')",
            [],
        ).expect("category");
        connection.execute(
            "INSERT INTO locations(id, name, code, created_at, updated_at) VALUES ('location', 'Depósito', 'DEP', 'now', 'now')",
            [],
        ).expect("location");
        connection.execute(
            "INSERT INTO items(id, asset_tag, name, category_id, location_id, quantity, minimum_quantity, created_at, updated_at) VALUES ('legacy', 'CABO-01', 'Cabo HDMI', 'category', 'location', 7, 2, 'now', 'now')",
            [],
        ).expect("legacy item");
        connection
            .execute_batch(MIGRATION_0002)
            .expect("migration 2");
        let migrated: (String, i64) = connection.query_row(
            "SELECT p.tracking_type, sb.quantity FROM products p JOIN stock_balances sb ON sb.product_id = p.id WHERE p.id = 'legacy'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ).expect("migrated product");
        assert_eq!(migrated, ("quantity".into(), 7));
    }

    #[test]
    fn creates_updates_and_deactivates_unused_category() {
        let database = Database::open(PathBuf::from(":memory:")).expect("database");
        let created = database
            .save_category(
                SaveMasterDataInput {
                    id: None,
                    name: "Acessórios".into(),
                    code: "acessorios".into(),
                },
                ADMIN_ID,
            )
            .expect("create category");
        assert_eq!(created.code, "ACESSORIOS");
        let updated = database
            .save_category(
                SaveMasterDataInput {
                    id: Some(created.id.clone()),
                    name: "Acessórios de TI".into(),
                    code: "ACESSORIOS_TI".into(),
                },
                ADMIN_ID,
            )
            .expect("update category");
        assert_eq!(updated.name, "Acessórios de TI");
        let inactive = database
            .set_category_active(&created.id, false, ADMIN_ID)
            .expect("deactivate category");
        assert!(!inactive.active);
    }

    #[test]
    fn blocks_deactivation_of_location_with_inventory() {
        let database = Database::open(PathBuf::from(":memory:")).expect("database");
        let location = database
            .list_location_records()
            .expect("locations")
            .into_iter()
            .find(|record| record.usage_count > 0)
            .expect("used location");
        let result = database.set_location_active(&location.id, false, ADMIN_ID);
        assert!(matches!(result, Err(AppError::Validation(_))));
    }

    #[test]
    fn creates_catalog_product_without_initial_balance() {
        let database = Database::open(PathBuf::from(":memory:")).expect("database");
        let item = database
            .create_item(
                CreateItemInput {
                    sku: "TEST-01".into(),
                    name: "Item de teste".into(),
                    category: "Testes".into(),
                    tracking_type: "quantity".into(),
                    serial_number_policy: "not_applicable".into(),
                    minimum_quantity: 1,
                    location_id: None,
                    initial_quantity: 0,
                    asset_tag: None,
                    serial_number: None,
                },
                ADMIN_ID,
            )
            .expect("item");
        assert_eq!(item.quantity, 0);
    }

    #[test]
    fn rejects_exit_above_available_balance() {
        let database = Database::open(PathBuf::from(":memory:")).expect("database");
        let product = quantity_product(&database);
        let location = database.list_locations().expect("locations")[0].id.clone();
        let result = database.create_movement(
            CreateMovementInput {
                product_id: product.id,
                kind: "exit".into(),
                quantity: 9_999,
                from_location_id: Some(location),
                to_location_id: None,
                asset_id: None,
                asset_tag: None,
                serial_number: None,
                asset_status: None,
                note: "teste".into(),
            },
            ADMIN_ID,
        );
        assert!(matches!(result, Err(AppError::Validation(_))));
    }

    #[test]
    fn transfers_serialized_asset_without_changing_total() {
        let database = Database::open(PathBuf::from(":memory:")).expect("database");
        let product = database
            .list_items("")
            .expect("list")
            .into_iter()
            .find(|item| item.tracking_type == "serialized")
            .expect("serialized product");
        let asset = database.list_assets(&product.id).expect("assets")[0]
            .id
            .clone();
        let locations = database.list_locations().expect("locations");
        let target = locations
            .iter()
            .find(|location| location.name == "Filial Sul")
            .expect("target")
            .id
            .clone();
        database
            .create_movement(
                CreateMovementInput {
                    product_id: product.id.clone(),
                    kind: "transfer".into(),
                    quantity: 1,
                    from_location_id: None,
                    to_location_id: Some(target.clone()),
                    asset_id: Some(asset.clone()),
                    asset_tag: None,
                    serial_number: None,
                    asset_status: None,
                    note: "teste".into(),
                },
                ADMIN_ID,
            )
            .expect("movement");
        let moved = database.list_assets(&product.id).expect("assets");
        assert_eq!(
            moved
                .iter()
                .find(|item| item.id == asset)
                .expect("asset")
                .location_id,
            target
        );
        assert_eq!(
            database.get_item(&product.id).expect("product").quantity,
            product.quantity
        );
    }

    #[test]
    fn filters_and_updates_asset_with_transactional_history() {
        let database = Database::open(PathBuf::from(":memory:")).expect("database");
        let asset = database
            .list_asset_records(AssetFilterInput {
                search: "NB-0014".into(),
                location_id: None,
                status: Some("available".into()),
            })
            .expect("filtered assets")
            .into_iter()
            .next()
            .expect("asset");
        let target = database
            .list_locations()
            .expect("locations")
            .into_iter()
            .find(|location| location.id != asset.location_id)
            .expect("target location");
        let movement_count_before: i64 = database
            .connection
            .lock()
            .query_row("SELECT COUNT(*) FROM stock_movements", [], |row| row.get(0))
            .expect("movement count");

        let updated = database
            .update_asset(
                UpdateAssetInput {
                    id: asset.id.clone(),
                    location_id: target.id.clone(),
                    status: "maintenance".into(),
                    note: "Revisão preventiva".into(),
                },
                ADMIN_ID,
            )
            .expect("update asset");

        assert_eq!(updated.location_id, target.id);
        assert_eq!(updated.status, "maintenance");
        let connection = database.connection.lock();
        let movement_count_after: i64 = connection
            .query_row("SELECT COUNT(*) FROM stock_movements", [], |row| row.get(0))
            .expect("movement count");
        let audit_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM audit_logs WHERE entity_type = 'asset' AND entity_id = ?1 AND action = 'update'",
                [&asset.id],
                |row| row.get(0),
            )
            .expect("audit count");
        assert_eq!(movement_count_after, movement_count_before + 2);
        assert_eq!(audit_count, 1);
    }

    #[test]
    fn operator_updates_operational_asset_status_only_inside_assigned_location() {
        let database = Database::open(PathBuf::from(":memory:")).expect("database");
        let operator_id = "123e4567-e89b-42d3-a456-426614174120";
        insert_test_user(&database, operator_id, "operador.ativo", "operator");
        let operator = AuthenticatedUser {
            id: operator_id.into(),
            username: "operador.ativo".into(),
            display_name: "Operador de ativos".into(),
            role: UserRole::Operator,
        };
        let asset = database
            .list_asset_records(AssetFilterInput {
                search: "NB-0014".into(),
                location_id: None,
                status: Some("available".into()),
            })
            .expect("available asset")
            .into_iter()
            .next()
            .expect("asset");

        let updated = database
            .update_asset_for(
                UpdateAssetInput {
                    id: asset.id.clone(),
                    location_id: asset.location_id.clone(),
                    status: "in_use".into(),
                    note: "Equipamento entregue ao usuário responsável".into(),
                },
                &operator,
            )
            .expect("operational status update");
        assert_eq!(updated.status, "in_use");

        let forbidden = database.update_asset_for(
            UpdateAssetInput {
                id: asset.id,
                location_id: asset.location_id,
                status: "maintenance".into(),
                note: "Tentativa de manutenção direta".into(),
            },
            &operator,
        );
        assert!(matches!(forbidden, Err(AppError::Forbidden(_))));
    }

    #[test]
    fn blocks_direct_changes_to_disposed_asset() {
        let database = Database::open(PathBuf::from(":memory:")).expect("database");
        let product = database
            .list_items("")
            .expect("products")
            .into_iter()
            .find(|item| item.tracking_type == "serialized")
            .expect("serialized product");
        let asset = database.list_assets(&product.id).expect("assets")[0]
            .id
            .clone();
        let location = database.list_assets(&product.id).expect("assets")[0]
            .location_id
            .clone();
        database
            .create_movement(
                CreateMovementInput {
                    product_id: product.id,
                    kind: "exit".into(),
                    quantity: 1,
                    from_location_id: None,
                    to_location_id: None,
                    asset_id: Some(asset.clone()),
                    asset_tag: None,
                    serial_number: None,
                    asset_status: None,
                    note: "Baixa de teste".into(),
                },
                ADMIN_ID,
            )
            .expect("dispose asset");

        let result = database.update_asset(
            UpdateAssetInput {
                id: asset,
                location_id: location,
                status: "available".into(),
                note: "tentativa".into(),
            },
            ADMIN_ID,
        );
        assert!(matches!(result, Err(AppError::Validation(_))));
    }

    #[test]
    fn lists_audit_events_with_entity_name_and_filters() {
        let database = Database::open(PathBuf::from(":memory:")).expect("database");
        let category = database
            .save_category(
                SaveMasterDataInput {
                    id: None,
                    name: "Telefonia".into(),
                    code: "TELEFONIA".into(),
                },
                ADMIN_ID,
            )
            .expect("create category");
        let logs = database
            .list_audit_logs(AuditFilterInput {
                search: "Telefonia".into(),
                entity_type: Some("category".into()),
                action: Some("create".into()),
                date_from: None,
                date_to: None,
            })
            .expect("audit logs");

        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].entity_id, category.id);
        assert_eq!(logs[0].entity_name, "Telefonia");
        assert_eq!(logs[0].actor, "Administrador");
        assert!(logs[0].after_data.is_some());
    }

    #[test]
    fn rejects_inverted_audit_date_filter() {
        let database = Database::open(PathBuf::from(":memory:")).expect("database");
        let result = database.list_audit_logs(AuditFilterInput {
            search: String::new(),
            entity_type: None,
            action: None,
            date_from: Some("2026-08-30".into()),
            date_to: Some("2026-08-29".into()),
        });
        assert!(matches!(result, Err(AppError::Validation(_))));
    }

    #[test]
    fn approves_quantity_adjustment_with_requester_reviewer_and_audit() {
        let database = Database::open(PathBuf::from(":memory:")).expect("database");
        let operator_id = "123e4567-e89b-42d3-a456-426614174101";
        let manager_id = "123e4567-e89b-42d3-a456-426614174102";
        insert_test_user(&database, operator_id, "operador.teste", "operator");
        insert_test_user(&database, manager_id, "gestor.teste", "manager");
        let product = quantity_product(&database);
        let location = database.list_locations().expect("locations")[0].id.clone();
        let before: i64 = {
            let connection = database.connection.lock();
            connection
                .query_row(
                    "SELECT COALESCE((SELECT quantity FROM stock_balances WHERE product_id = ?1 AND location_id = ?2), 0)",
                    params![product.id, location],
                    |row| row.get(0),
                )
                .expect("balance")
        };
        let request = database
            .create_adjustment_request(
                CreateAdjustmentRequestInput {
                    product_id: product.id.clone(),
                    asset_id: None,
                    kind: "quantity_adjustment".into(),
                    location_id: Some(location.clone()),
                    requested_quantity: Some(before + 2),
                    requested_status: None,
                    requested_location_id: None,
                    description: "Contagem física conferida por duas pessoas".into(),
                },
                operator_id,
            )
            .expect("request");
        assert_eq!(request.status, "pending");
        assert_eq!(request.requester_id, operator_id);

        let reviewed = database
            .review_adjustment_request(
                ReviewAdjustmentRequestInput {
                    id: request.id.clone(),
                    decision: "approved".into(),
                    note: "Evidências conferidas e ajuste autorizado".into(),
                    version: request.version,
                },
                manager_id,
            )
            .expect("approval");
        assert_eq!(reviewed.status, "approved");
        assert_eq!(reviewed.reviewer_id.as_deref(), Some(manager_id));
        let connection = database.connection.lock();
        let after: i64 = connection
            .query_row(
                "SELECT quantity FROM stock_balances WHERE product_id = ?1 AND location_id = ?2",
                params![product.id, location],
                |row| row.get(0),
            )
            .expect("balance");
        assert_eq!(after, before + 2);
        let approval_audit: (String, String) = connection
            .query_row(
                "SELECT actor_id, json_extract(after_data, '$.requesterId') FROM audit_logs WHERE entity_type = 'adjustment_request' AND entity_id = ?1 AND action = 'approve'",
                [&request.id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("approval audit");
        assert_eq!(approval_audit, (manager_id.into(), operator_id.into()));
    }

    #[test]
    fn blocks_requester_from_reviewing_own_adjustment() {
        let database = Database::open(PathBuf::from(":memory:")).expect("database");
        let manager_id = "123e4567-e89b-42d3-a456-426614174103";
        insert_test_user(&database, manager_id, "gestor.solicitante", "manager");
        let product = quantity_product(&database);
        let location = database.list_locations().expect("locations")[0].id.clone();
        let request = database
            .create_adjustment_request(
                CreateAdjustmentRequestInput {
                    product_id: product.id,
                    asset_id: None,
                    kind: "quantity_adjustment".into(),
                    location_id: Some(location),
                    requested_quantity: Some(42),
                    requested_status: None,
                    requested_location_id: None,
                    description: "Solicitação criada para validar segregação".into(),
                },
                manager_id,
            )
            .expect("request");
        let result = database.review_adjustment_request(
            ReviewAdjustmentRequestInput {
                id: request.id,
                decision: "rejected".into(),
                note: "Parecer de teste".into(),
                version: request.version,
            },
            manager_id,
        );
        assert!(matches!(result, Err(AppError::Forbidden(_))));
    }

    #[test]
    fn creates_and_reviews_replenishment_with_reserved_transfer_stock() {
        let database = Database::open(PathBuf::from(":memory:")).expect("database");
        let operator_id = "123e4567-e89b-42d3-a456-426614174111";
        let manager_id = "123e4567-e89b-42d3-a456-426614174112";
        insert_test_user(&database, operator_id, "operador.reposicao", "operator");
        insert_test_user(&database, manager_id, "gestor.reposicao", "manager");
        let operator = AuthenticatedUser {
            id: operator_id.into(),
            username: "operador.reposicao".into(),
            display_name: "Operador reposição".into(),
            role: UserRole::Operator,
        };
        let manager = AuthenticatedUser {
            id: manager_id.into(),
            username: "gestor.reposicao".into(),
            display_name: "Gestor reposição".into(),
            role: UserRole::Manager,
        };
        let product = quantity_product(&database);
        let locations = database.list_locations().expect("locations");
        let source = locations
            .iter()
            .find(|location| {
                database.connection.lock().query_row(
                    "SELECT COALESCE((SELECT quantity FROM stock_balances WHERE product_id = ?1 AND location_id = ?2), 0)",
                    params![product.id, location.id],
                    |row| row.get::<_, i64>(0),
                ).unwrap_or(0) > 0
            })
            .expect("source")
            .id
            .clone();
        let destination = locations
            .iter()
            .find(|location| location.id != source)
            .expect("destination")
            .id
            .clone();
        let request = database
            .create_replenishment_request(
                CreateReplenishmentRequestInput {
                    destination_location_id: destination,
                    priority: "high".into(),
                    justification: "Unidade sem periféricos suficientes para atendimento".into(),
                    items: vec![crate::models::CreateReplenishmentItemInput {
                        product_id: product.id.clone(),
                        quantity: 1,
                    }],
                },
                &operator,
            )
            .expect("request");
        assert_eq!(request.status, "pending");
        assert_eq!(request.items[0].stock_snapshot, 0);
        assert_eq!(request.items[0].received_quantity, 0);
        assert_eq!(request.items[0].serial_number_policy, "not_applicable");

        let reviewed = database
            .review_replenishment_request(
                ReviewReplenishmentRequestInput {
                    id: request.id.clone(),
                    note: "Transferência autorizada após conferência do saldo disponível".into(),
                    version: request.version,
                    items: vec![crate::models::ReplenishmentDecisionItemInput {
                        item_id: request.items[0].id.clone(),
                        transfer_quantity: 1,
                        purchase_quantity: 0,
                        source_location_id: Some(source.clone()),
                        purchase_reference: None,
                    }],
                },
                &manager,
            )
            .expect("review");
        assert_eq!(reviewed.status, "approved");
        let reserved: i64 = database
            .connection
            .lock()
            .query_row(
                "SELECT reserved_quantity FROM stock_balances WHERE product_id = ?1 AND location_id = ?2",
                params![product.id, source],
                |row| row.get(0),
            )
            .expect("reserved stock");
        assert_eq!(reserved, 1);
    }

    #[test]
    fn dispatches_receives_and_retries_divergent_quantity_transfer() {
        let database = Database::open(PathBuf::from(":memory:")).expect("database");
        let operator_id = "123e4567-e89b-42d3-a456-426614174131";
        let manager_id = "123e4567-e89b-42d3-a456-426614174132";
        insert_test_user(&database, operator_id, "operador.destino", "operator");
        insert_test_user(&database, manager_id, "gestor.origem", "manager");
        let operator = AuthenticatedUser {
            id: operator_id.into(),
            username: "operador.destino".into(),
            display_name: "Operador destino".into(),
            role: UserRole::Operator,
        };
        let manager = AuthenticatedUser {
            id: manager_id.into(),
            username: "gestor.origem".into(),
            display_name: "Gestor origem".into(),
            role: UserRole::Manager,
        };
        let product = database
            .list_items("")
            .expect("products")
            .into_iter()
            .find(|item| item.tracking_type == "quantity" && item.quantity >= 2)
            .expect("quantity stock");
        let locations = database.list_locations().expect("locations");
        let source = locations.iter().find(|location| database.connection.lock().query_row(
            "SELECT COALESCE((SELECT quantity FROM stock_balances WHERE product_id = ?1 AND location_id = ?2), 0)",
            params![product.id, location.id], |row| row.get::<_, i64>(0)).unwrap_or(0) >= 2).expect("source").id.clone();
        let destination = locations
            .iter()
            .find(|location| location.id != source)
            .expect("destination")
            .id
            .clone();
        let source_before = database
            .connection
            .lock()
            .query_row(
                "SELECT quantity FROM stock_balances WHERE product_id = ?1 AND location_id = ?2",
                params![product.id, source],
                |row| row.get::<_, i64>(0),
            )
            .expect("source balance");
        let destination_before = database.connection.lock().query_row(
            "SELECT COALESCE((SELECT quantity FROM stock_balances WHERE product_id = ?1 AND location_id = ?2), 0)",
            params![product.id, destination], |row| row.get::<_, i64>(0)).expect("destination balance");
        let request = database
            .create_replenishment_request(
                CreateReplenishmentRequestInput {
                    destination_location_id: destination.clone(),
                    priority: "high".into(),
                    justification: "Reposição por transferência com confirmação no destino".into(),
                    items: vec![crate::models::CreateReplenishmentItemInput {
                        product_id: product.id.clone(),
                        quantity: 2,
                    }],
                },
                &operator,
            )
            .expect("request");
        let reviewed = database
            .review_replenishment_request(
                ReviewReplenishmentRequestInput {
                    id: request.id.clone(),
                    note: "Transferência integral autorizada com saldo reservado".into(),
                    version: request.version,
                    items: vec![crate::models::ReplenishmentDecisionItemInput {
                        item_id: request.items[0].id.clone(),
                        transfer_quantity: 2,
                        purchase_quantity: 0,
                        source_location_id: Some(source.clone()),
                        purchase_reference: None,
                    }],
                },
                &manager,
            )
            .expect("review");
        let first_dispatch = database
            .dispatch_replenishment_transfer(
                DispatchReplenishmentTransferInput {
                    request_id: request.id.clone(),
                    reference: "GUIA-001".into(),
                    note: "Despacho inicial".into(),
                    items: vec![crate::models::DispatchReplenishmentTransferItemInput {
                        request_item_id: reviewed.items[0].id.clone(),
                        quantity: 2,
                        asset_ids: vec![],
                    }],
                },
                &manager,
            )
            .expect("dispatch");
        assert_eq!(first_dispatch.status, "dispatched");
        assert_eq!(database.connection.lock().query_row(
            "SELECT quantity FROM stock_balances WHERE product_id = ?1 AND location_id = ?2",
            params![product.id, source], |row| row.get::<_, i64>(0)).unwrap(), source_before - 2);
        assert_eq!(database.connection.lock().query_row(
            "SELECT COALESCE((SELECT quantity FROM stock_balances WHERE product_id = ?1 AND location_id = ?2), 0)",
            params![product.id, destination], |row| row.get::<_, i64>(0)).unwrap(), destination_before);

        let divergent = database
            .receive_replenishment_transfer(
                ReceiveReplenishmentTransferInput {
                    shipment_id: first_dispatch.id.clone(),
                    reference: "REC-001".into(),
                    note: "Conferência parcial".into(),
                    version: first_dispatch.version,
                    items: vec![crate::models::ReceiveReplenishmentTransferItemInput {
                        shipment_item_id: first_dispatch.items[0].id.clone(),
                        received_quantity: 1,
                        rejected_quantity: 1,
                        note: "Uma unidade chegou avariada".into(),
                    }],
                },
                &operator,
            )
            .expect("divergent receipt");
        assert_eq!(divergent.status, "divergent");
        let progress = database
            .get_replenishment_request(&request.id)
            .expect("progress");
        assert_eq!(progress.items[0].transfer_received_quantity, 1);
        assert_eq!(progress.items[0].transfer_rejected_quantity, 1);
        assert_eq!(progress.items[0].transfer_in_transit_quantity, 0);
        assert_eq!(progress.status, "in_fulfillment");

        let retry = database
            .dispatch_replenishment_transfer(
                DispatchReplenishmentTransferInput {
                    request_id: request.id.clone(),
                    reference: "GUIA-002".into(),
                    note: "Reenvio da unidade recusada".into(),
                    items: vec![crate::models::DispatchReplenishmentTransferItemInput {
                        request_item_id: progress.items[0].id.clone(),
                        quantity: 1,
                        asset_ids: vec![],
                    }],
                },
                &manager,
            )
            .expect("retry dispatch");
        database
            .receive_replenishment_transfer(
                ReceiveReplenishmentTransferInput {
                    shipment_id: retry.id.clone(),
                    reference: "REC-002".into(),
                    note: "Reposição conferida".into(),
                    version: retry.version,
                    items: vec![crate::models::ReceiveReplenishmentTransferItemInput {
                        shipment_item_id: retry.items[0].id.clone(),
                        received_quantity: 1,
                        rejected_quantity: 0,
                        note: String::new(),
                    }],
                },
                &operator,
            )
            .expect("retry receipt");
        let completed = database
            .get_replenishment_request(&request.id)
            .expect("completed");
        assert_eq!(completed.items[0].transfer_received_quantity, 2);
        assert_eq!(completed.status, "fulfilled");
        assert_eq!(database.connection.lock().query_row(
            "SELECT quantity FROM stock_balances WHERE product_id = ?1 AND location_id = ?2",
            params![product.id, destination], |row| row.get::<_, i64>(0)).unwrap(), destination_before + 2);
    }

    #[test]
    fn reserves_dispatches_and_receives_a_specific_serialized_asset() {
        let database = Database::open(PathBuf::from(":memory:")).expect("database");
        let operator_id = "123e4567-e89b-42d3-a456-426614174141";
        let manager_id = "123e4567-e89b-42d3-a456-426614174142";
        insert_test_user(&database, operator_id, "operador.patrimonio", "operator");
        insert_test_user(&database, manager_id, "gestor.patrimonio", "manager");
        let operator = AuthenticatedUser {
            id: operator_id.into(),
            username: "operador.patrimonio".into(),
            display_name: "Operador patrimônio".into(),
            role: UserRole::Operator,
        };
        let manager = AuthenticatedUser {
            id: manager_id.into(),
            username: "gestor.patrimonio".into(),
            display_name: "Gestor patrimônio".into(),
            role: UserRole::Manager,
        };
        let asset = database
            .list_asset_records(AssetFilterInput {
                search: "NB-0014".into(),
                location_id: None,
                status: Some("available".into()),
            })
            .unwrap()
            .remove(0);
        let destination = database
            .list_locations()
            .unwrap()
            .into_iter()
            .find(|location| location.id != asset.location_id)
            .unwrap();
        let request = database
            .create_replenishment_request(
                CreateReplenishmentRequestInput {
                    destination_location_id: destination.id.clone(),
                    priority: "normal".into(),
                    justification: "Transferência patrimonial necessária para a unidade".into(),
                    items: vec![crate::models::CreateReplenishmentItemInput {
                        product_id: asset.product_id.clone(),
                        quantity: 1,
                    }],
                },
                &operator,
            )
            .unwrap();
        let reviewed = database
            .review_replenishment_request(
                ReviewReplenishmentRequestInput {
                    id: request.id.clone(),
                    note: "Patrimônio disponível reservado automaticamente".into(),
                    version: request.version,
                    items: vec![crate::models::ReplenishmentDecisionItemInput {
                        item_id: request.items[0].id.clone(),
                        transfer_quantity: 1,
                        purchase_quantity: 0,
                        source_location_id: Some(asset.location_id.clone()),
                        purchase_reference: None,
                    }],
                },
                &manager,
            )
            .unwrap();
        assert_eq!(reviewed.items[0].reserved_assets[0].id, asset.id);
        let shipment = database
            .dispatch_replenishment_transfer(
                DispatchReplenishmentTransferInput {
                    request_id: request.id.clone(),
                    reference: "GUIA-PAT-01".into(),
                    note: "Ativo lacrado para transporte".into(),
                    items: vec![crate::models::DispatchReplenishmentTransferItemInput {
                        request_item_id: reviewed.items[0].id.clone(),
                        quantity: 1,
                        asset_ids: vec![asset.id.clone()],
                    }],
                },
                &manager,
            )
            .unwrap();
        let in_transit: Option<String> = database
            .connection
            .lock()
            .query_row(
                "SELECT in_transit_shipment_id FROM assets WHERE id = ?1",
                [&asset.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(in_transit.as_deref(), Some(shipment.id.as_str()));
        database
            .receive_replenishment_transfer(
                ReceiveReplenishmentTransferInput {
                    shipment_id: shipment.id.clone(),
                    reference: "REC-PAT-01".into(),
                    note: "Patrimônio e série conferidos".into(),
                    version: shipment.version,
                    items: vec![crate::models::ReceiveReplenishmentTransferItemInput {
                        shipment_item_id: shipment.items[0].id.clone(),
                        received_quantity: 1,
                        rejected_quantity: 0,
                        note: String::new(),
                    }],
                },
                &operator,
            )
            .unwrap();
        let (location, transit): (String, Option<String>) = database
            .connection
            .lock()
            .query_row(
                "SELECT location_id, in_transit_shipment_id FROM assets WHERE id = ?1",
                [&asset.id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(location, destination.id);
        assert!(transit.is_none());
        assert_eq!(
            database
                .get_replenishment_request(&request.id)
                .unwrap()
                .status,
            "fulfilled"
        );
    }

    #[test]
    fn creates_atomic_serialized_receipt_batch_with_unique_traceability() {
        let database = Database::open(PathBuf::from(":memory:")).expect("database");
        let admin = AuthenticatedUser {
            id: ADMIN_ID.into(),
            username: "admin".into(),
            display_name: "Administrador".into(),
            role: UserRole::Admin,
        };
        let product = database
            .list_items("")
            .expect("products")
            .into_iter()
            .find(|item| item.tracking_type == "serialized")
            .expect("serialized product");
        let location = database.list_locations().expect("locations")[0].id.clone();
        let item = |tag: &str, serial: Option<&str>| CreateMovementInput {
            product_id: product.id.clone(),
            kind: "entry".into(),
            quantity: 1,
            from_location_id: None,
            to_location_id: Some(location.clone()),
            asset_id: None,
            asset_tag: Some(tag.into()),
            serial_number: serial.map(str::to_owned),
            asset_status: None,
            note: "Recebimento de teste".into(),
        };
        let batch = database
            .create_movement_batch(
                CreateMovementBatchInput {
                    kind: "entry".into(),
                    reference: "NF-TESTE".into(),
                    note: "Dois ativos individualizados".into(),
                    replenishment_request_id: None,
                    items: vec![
                        item("BATCH-NB-01", Some("SER-BATCH-01")),
                        item("BATCH-NB-02", Some("SER-BATCH-02")),
                    ],
                },
                &admin,
            )
            .expect("batch");
        assert_eq!(batch.movements.len(), 2);
        let linked: i64 = database
            .connection
            .lock()
            .query_row(
                "SELECT COUNT(*) FROM assets WHERE receipt_batch_id = ?1",
                [&batch.id],
                |row| row.get(0),
            )
            .expect("linked assets");
        assert_eq!(linked, 2);

        let before: i64 = database
            .connection
            .lock()
            .query_row("SELECT COUNT(*) FROM movement_batches", [], |row| {
                row.get(0)
            })
            .expect("batch count");
        let invalid = database.create_movement_batch(
            CreateMovementBatchInput {
                kind: "entry".into(),
                reference: "NF-INVALIDA".into(),
                note: "Deve reverter integralmente".into(),
                replenishment_request_id: None,
                items: vec![item("BATCH-NB-03", None)],
            },
            &admin,
        );
        assert!(matches!(invalid, Err(AppError::Validation(_))));
        let after: i64 = database
            .connection
            .lock()
            .query_row("SELECT COUNT(*) FROM movement_batches", [], |row| {
                row.get(0)
            })
            .expect("batch count");
        assert_eq!(after, before);
    }

    #[test]
    fn receives_only_the_remaining_approved_purchase_and_exposes_progress() {
        let database = Database::open(PathBuf::from(":memory:")).expect("database");
        let operator_id = "123e4567-e89b-42d3-a456-426614174140";
        let manager_id = "123e4567-e89b-42d3-a456-426614174141";
        insert_test_user(&database, operator_id, "operador.recebimento", "operator");
        insert_test_user(&database, manager_id, "gestor.recebimento", "manager");
        let operator = AuthenticatedUser {
            id: operator_id.into(),
            username: "operador.recebimento".into(),
            display_name: "Operador".into(),
            role: UserRole::Operator,
        };
        let manager = AuthenticatedUser {
            id: manager_id.into(),
            username: "gestor.recebimento".into(),
            display_name: "Gestor".into(),
            role: UserRole::Manager,
        };
        let product = quantity_product(&database);
        let destination = database.list_locations().expect("locations")[0].id.clone();
        let request = database
            .create_replenishment_request(
                CreateReplenishmentRequestInput {
                    destination_location_id: destination.clone(),
                    priority: "normal".into(),
                    justification: "Reposição necessária para validar recebimento parcial".into(),
                    items: vec![crate::models::CreateReplenishmentItemInput {
                        product_id: product.id.clone(),
                        quantity: 2,
                    }],
                },
                &operator,
            )
            .expect("request");
        let reviewed = database
            .review_replenishment_request(
                ReviewReplenishmentRequestInput {
                    id: request.id.clone(),
                    note: "Compra integral aprovada pelo gestor responsável".into(),
                    version: request.version,
                    items: vec![crate::models::ReplenishmentDecisionItemInput {
                        item_id: request.items[0].id.clone(),
                        transfer_quantity: 0,
                        purchase_quantity: 2,
                        source_location_id: None,
                        purchase_reference: Some("OC-RECEBIMENTO".into()),
                    }],
                },
                &manager,
            )
            .expect("review");
        let movement = || CreateMovementInput {
            product_id: product.id.clone(),
            kind: "entry".into(),
            quantity: 1,
            from_location_id: None,
            to_location_id: Some(destination.clone()),
            asset_id: None,
            asset_tag: None,
            serial_number: None,
            asset_status: None,
            note: "Recebimento parcial".into(),
        };
        let missing_reference = database.create_movement_batch(
            CreateMovementBatchInput {
                kind: "entry".into(),
                reference: "".into(),
                note: "Sem documento".into(),
                replenishment_request_id: Some(request.id.clone()),
                items: vec![movement()],
            },
            &manager,
        );
        assert!(matches!(missing_reference, Err(AppError::Validation(_))));

        database
            .create_movement_batch(
                CreateMovementBatchInput {
                    kind: "entry".into(),
                    reference: "NF-PARCIAL".into(),
                    note: "Primeira entrega".into(),
                    replenishment_request_id: Some(request.id.clone()),
                    items: vec![movement()],
                },
                &manager,
            )
            .expect("partial receipt");
        let partial = database
            .get_replenishment_request(&request.id)
            .expect("partial request");
        assert_eq!(partial.status, "in_fulfillment");
        assert_eq!(partial.items[0].received_quantity, 1);
        assert_eq!(partial.items[0].serial_number_policy, "not_applicable");

        database
            .create_movement_batch(
                CreateMovementBatchInput {
                    kind: "entry".into(),
                    reference: "NF-FINAL".into(),
                    note: "Entrega final".into(),
                    replenishment_request_id: Some(reviewed.id),
                    items: vec![movement()],
                },
                &manager,
            )
            .expect("final receipt");
        let fulfilled = database
            .get_replenishment_request(&request.id)
            .expect("fulfilled request");
        assert_eq!(fulfilled.status, "fulfilled");
        assert_eq!(fulfilled.items[0].received_quantity, 2);
    }

    #[test]
    fn reports_and_resolves_asset_incident_with_scope_and_history() {
        let database = Database::open(PathBuf::from(":memory:")).expect("database");
        let operator_id = "123e4567-e89b-42d3-a456-426614174130";
        let manager_id = "123e4567-e89b-42d3-a456-426614174131";
        insert_test_user(&database, operator_id, "operador.manutencao", "operator");
        insert_test_user(&database, manager_id, "gestor.manutencao", "manager");
        let operator = AuthenticatedUser {
            id: operator_id.into(),
            username: "operador.manutencao".into(),
            display_name: "Operador".into(),
            role: UserRole::Operator,
        };
        let manager = AuthenticatedUser {
            id: manager_id.into(),
            username: "gestor.manutencao".into(),
            display_name: "Gestor".into(),
            role: UserRole::Manager,
        };
        let asset = database
            .list_asset_records(AssetFilterInput {
                search: "NB-0014".into(),
                location_id: None,
                status: Some("available".into()),
            })
            .expect("assets")
            .into_iter()
            .next()
            .expect("asset");
        let incident = database
            .create_asset_incident(
                CreateAssetIncidentInput {
                    asset_id: asset.id.clone(),
                    custodian_name: Some("Maria".into()),
                    description: "Notebook não liga após ser devolvido".into(),
                },
                &operator,
            )
            .expect("incident");
        assert_eq!(incident.status, "pending");
        assert_eq!(
            database.get_asset_record(&asset.id).expect("asset").status,
            "maintenance"
        );
        let resolved = database
            .review_asset_incident(
                ReviewAssetIncidentInput {
                    id: incident.id,
                    status: "resolved".into(),
                    resolution_note: "Reparo concluído e equipamento testado".into(),
                    version: incident.version,
                },
                &manager,
            )
            .expect("resolution");
        assert_eq!(resolved.status, "resolved");
        assert_eq!(
            database.get_asset_record(&asset.id).expect("asset").status,
            "available"
        );
    }

    #[test]
    fn restores_valid_backup_and_creates_safety_copy() {
        let (database, directory) = file_database();
        database
            .save_category(
                SaveMasterDataInput {
                    id: None,
                    name: "Estado original".into(),
                    code: "ESTADO_ORIGINAL".into(),
                },
                ADMIN_ID,
            )
            .expect("original category");
        let backup = database.create_backup().expect("backup");
        assert_file_is_encrypted(
            &database
                .backup_directory()
                .expect("backup directory")
                .join(&backup.file_name),
        );
        database
            .save_category(
                SaveMasterDataInput {
                    id: None,
                    name: "Criada depois".into(),
                    code: "CRIADA_DEPOIS".into(),
                },
                ADMIN_ID,
            )
            .expect("later category");

        let result = database
            .restore_backup(&backup.file_name, ADMIN_ID)
            .expect("restore backup");

        let categories = database.list_categories().expect("categories");
        assert!(categories.iter().any(|item| item.name == "Estado original"));
        assert!(!categories.iter().any(|item| item.name == "Criada depois"));
        assert!(result.safety_backup.starts_with("pre-restore-"));
        assert!(database
            .list_backups()
            .expect("backups")
            .iter()
            .any(|item| item.file_name == result.safety_backup && item.valid));
        let restore_logs = database
            .list_audit_logs(AuditFilterInput {
                search: backup.file_name,
                entity_type: Some("backup".into()),
                action: Some("restore".into()),
                date_from: None,
                date_to: None,
            })
            .expect("restore audit");
        assert_eq!(restore_logs.len(), 1);
        drop(database);
        fs::remove_dir_all(directory).expect("cleanup");
    }

    #[test]
    fn rejects_backup_outside_internal_directory() {
        let (database, directory) = file_database();
        let result = database.restore_backup("..\\foreign.db", ADMIN_ID);
        assert!(matches!(result, Err(AppError::Validation(_))));
        drop(database);
        fs::remove_dir_all(directory).expect("cleanup");
    }

    #[test]
    fn identifies_corrupted_internal_backup() {
        let (database, directory) = file_database();
        let backup_directory = database.backup_directory().expect("backup directory");
        fs::create_dir_all(&backup_directory).expect("create backup directory");
        fs::write(
            backup_directory.join("corrupted.db"),
            b"not a sqlite database",
        )
        .expect("corrupted backup");
        let backups = database.list_backups().expect("backups");
        assert!(
            !backups
                .iter()
                .find(|backup| backup.file_name == "corrupted.db")
                .expect("corrupted record")
                .valid
        );
        assert!(database.restore_backup("corrupted.db", ADMIN_ID).is_err());
        drop(database);
        fs::remove_dir_all(directory).expect("cleanup");
    }

    #[test]
    fn imports_products_assets_movements_audit_and_safety_backup() {
        let (database, directory) = file_database();
        let stage = StagedImport {
            file_name: "lote-validado.xlsx".into(),
            location_id: "2d6c9f1f-dbf5-4321-8500-000000000001".into(),
            products: vec![
                StagedProduct {
                    sku: "IMPORT-QTY-001".into(),
                    name: "Material importado".into(),
                    category: "Importados".into(),
                    tracking_type: "quantity".into(),
                    minimum_quantity: 2,
                    quantity: 7,
                    assets: Vec::new(),
                },
                StagedProduct {
                    sku: "IMPORT-SERIAL-001".into(),
                    name: "Ativo importado".into(),
                    category: "Importados".into(),
                    tracking_type: "serialized".into(),
                    minimum_quantity: 0,
                    quantity: 1,
                    assets: vec![StagedAsset {
                        asset_tag: "IMPORT-PAT-001".into(),
                        serial_number: Some("IMPORT-SN-001".into()),
                    }],
                },
            ],
        };

        let result = database.import_stage(stage, ADMIN_ID).expect("import");
        assert_eq!(result.products_created, 2);
        assert_eq!(result.assets_created, 1);
        assert_eq!(result.quantity_imported, 8);
        assert!(result.safety_backup.starts_with("pre-import-"));
        assert!(database.list_items("IMPORT-").expect("items").len() >= 2);
        assert!(
            database
                .list_audit_logs(AuditFilterInput {
                    search: "lote-validado".into(),
                    entity_type: Some("spreadsheet".into()),
                    action: Some("import".into()),
                    date_from: None,
                    date_to: None
                })
                .expect("audit")
                .len()
                == 1
        );
        assert!(database
            .list_backups()
            .expect("backups")
            .iter()
            .any(|backup| backup.kind == "import"));
        drop(database);
        fs::remove_dir_all(directory).expect("cleanup");
    }

    #[test]
    fn rolls_back_the_whole_import_when_database_rejects_one_asset() {
        let (database, directory) = file_database();
        let stage = StagedImport {
            file_name: "lote-com-colisao.xlsx".into(),
            location_id: "2d6c9f1f-dbf5-4321-8500-000000000001".into(),
            products: vec![StagedProduct {
                sku: "ROLLBACK-001".into(),
                name: "Produto que deve ser desfeito".into(),
                category: "Teste transacional".into(),
                tracking_type: "serialized".into(),
                minimum_quantity: 0,
                quantity: 2,
                assets: vec![
                    StagedAsset {
                        asset_tag: "COLISAO-001".into(),
                        serial_number: None,
                    },
                    StagedAsset {
                        asset_tag: "COLISAO-001".into(),
                        serial_number: None,
                    },
                ],
            }],
        };

        assert!(database.import_stage(stage, ADMIN_ID).is_err());
        assert!(database
            .list_items("ROLLBACK-001")
            .expect("items")
            .is_empty());
        assert!(database
            .list_audit_logs(AuditFilterInput {
                search: "lote-com-colisao".into(),
                entity_type: None,
                action: None,
                date_from: None,
                date_to: None
            })
            .expect("audit")
            .is_empty());
        drop(database);
        fs::remove_dir_all(directory).expect("cleanup");
    }
}
