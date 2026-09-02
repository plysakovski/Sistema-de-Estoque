use tauri::State;

use crate::{
    auth::{AuthService, Permission},
    database::Database,
    error::AppResult,
    imports::{ImportPreview, ImportResult, ImportService},
    models::{
        AdjustmentRequestView, AssetFilterInput, AssetIncidentView, AssetRecordView, AssetView,
        AuditFilterInput, AuditLogView, AuthStatus, AuthenticatedUser, BackupRecord,
        BootstrapAdminInput, ConfirmSensitiveActionInput, CreateAdjustmentRequestInput,
        CreateAssetIncidentInput, CreateItemInput, CreateMovementBatchInput, CreateMovementInput,
        CreateReplenishmentRequestInput, CreateUserInput, Dashboard, InventoryItem, LocationOption,
        LoginInput, MasterDataRecord, MovementBatchView, MovementView, RecoveryKeyReveal,
        RecoveryKeyStatus, ReplenishmentRequestView, ResetUserPasswordInput, RestoreResult,
        ReviewAdjustmentRequestInput, ReviewAssetIncidentInput, ReviewReplenishmentRequestInput,
        SaveMasterDataInput, UpdateAssetInput, UpdateUserInput, UserRole, UserView,
    },
};

#[tauri::command]
pub fn auth_status(
    database: State<'_, Database>,
    auth: State<'_, AuthService>,
) -> AppResult<AuthStatus> {
    auth.status(&database)
}

#[tauri::command]
pub fn bootstrap_admin(
    input: BootstrapAdminInput,
    database: State<'_, Database>,
    auth: State<'_, AuthService>,
) -> AppResult<AuthenticatedUser> {
    auth.bootstrap(&database, input)
}

#[tauri::command]
pub fn login(
    input: LoginInput,
    database: State<'_, Database>,
    auth: State<'_, AuthService>,
) -> AppResult<AuthenticatedUser> {
    auth.login(&database, input)
}

#[tauri::command]
pub fn logout(auth: State<'_, AuthService>) {
    auth.logout();
}

#[tauri::command]
pub fn get_dashboard(
    database: State<'_, Database>,
    auth: State<'_, AuthService>,
) -> AppResult<Dashboard> {
    let actor = auth.require(&database, Permission::ReadInventory)?;
    let mut dashboard = database.dashboard_for(&actor)?;
    if actor.role == UserRole::Operator {
        dashboard.pending_adjustment_count = 0;
    }
    Ok(dashboard)
}

#[tauri::command]
pub fn list_items(
    search: Option<String>,
    database: State<'_, Database>,
    auth: State<'_, AuthService>,
) -> AppResult<Vec<InventoryItem>> {
    let actor = auth.require(&database, Permission::ReadInventory)?;
    database.list_items_for(search.as_deref().unwrap_or_default(), &actor)
}

#[tauri::command]
pub fn create_item(
    input: CreateItemInput,
    database: State<'_, Database>,
    auth: State<'_, AuthService>,
) -> AppResult<InventoryItem> {
    let actor = auth.require(&database, Permission::ManageInventory)?;
    database.create_item_for(input, &actor)
}

#[tauri::command]
pub fn list_locations(
    database: State<'_, Database>,
    auth: State<'_, AuthService>,
) -> AppResult<Vec<LocationOption>> {
    let actor = auth.require(&database, Permission::ReadInventory)?;
    database.list_locations_for(&actor)
}

#[tauri::command]
pub fn list_location_records(
    database: State<'_, Database>,
    auth: State<'_, AuthService>,
) -> AppResult<Vec<MasterDataRecord>> {
    auth.require(&database, Permission::ManageLocations)?;
    database.list_location_records()
}

#[tauri::command]
pub fn save_location(
    input: SaveMasterDataInput,
    database: State<'_, Database>,
    auth: State<'_, AuthService>,
) -> AppResult<MasterDataRecord> {
    let actor = auth.require(&database, Permission::ManageLocations)?;
    database.save_location(input, &actor.id)
}

#[tauri::command]
pub fn set_location_active(
    id: String,
    active: bool,
    database: State<'_, Database>,
    auth: State<'_, AuthService>,
) -> AppResult<MasterDataRecord> {
    let actor = auth.require(&database, Permission::ManageLocations)?;
    database.set_location_active(&id, active, &actor.id)
}

#[tauri::command]
pub fn list_categories(
    database: State<'_, Database>,
    auth: State<'_, AuthService>,
) -> AppResult<Vec<MasterDataRecord>> {
    auth.require(&database, Permission::ReadInventory)?;
    database.list_categories()
}

#[tauri::command]
pub fn save_category(
    input: SaveMasterDataInput,
    database: State<'_, Database>,
    auth: State<'_, AuthService>,
) -> AppResult<MasterDataRecord> {
    let actor = auth.require(&database, Permission::ManageOperations)?;
    database.save_category(input, &actor.id)
}

#[tauri::command]
pub fn set_category_active(
    id: String,
    active: bool,
    database: State<'_, Database>,
    auth: State<'_, AuthService>,
) -> AppResult<MasterDataRecord> {
    let actor = auth.require(&database, Permission::ManageOperations)?;
    database.set_category_active(&id, active, &actor.id)
}

#[tauri::command]
pub fn list_assets(
    product_id: String,
    database: State<'_, Database>,
    auth: State<'_, AuthService>,
) -> AppResult<Vec<AssetView>> {
    let actor = auth.require(&database, Permission::ReadInventory)?;
    database.list_assets_for(&product_id, &actor)
}

#[tauri::command]
pub fn list_asset_records(
    filters: AssetFilterInput,
    database: State<'_, Database>,
    auth: State<'_, AuthService>,
) -> AppResult<Vec<AssetRecordView>> {
    let actor = auth.require(&database, Permission::ReadInventory)?;
    database.list_asset_records_for(filters, &actor)
}

#[tauri::command]
pub fn update_asset(
    input: UpdateAssetInput,
    database: State<'_, Database>,
    auth: State<'_, AuthService>,
) -> AppResult<AssetRecordView> {
    let actor = auth.require(&database, Permission::UpdateOperationalAssetStatus)?;
    database.update_asset_for(input, &actor)
}

#[tauri::command]
pub fn list_audit_logs(
    filters: AuditFilterInput,
    database: State<'_, Database>,
    auth: State<'_, AuthService>,
) -> AppResult<Vec<AuditLogView>> {
    auth.require(&database, Permission::ViewAudit)?;
    database.list_audit_logs(filters)
}

#[tauri::command]
pub fn list_movements(
    database: State<'_, Database>,
    auth: State<'_, AuthService>,
) -> AppResult<Vec<MovementView>> {
    let actor = auth.require(&database, Permission::ReadInventory)?;
    database.list_movements_for(&actor)
}

#[tauri::command]
pub fn create_movement(
    input: CreateMovementInput,
    database: State<'_, Database>,
    auth: State<'_, AuthService>,
) -> AppResult<MovementView> {
    let actor = auth.require(&database, Permission::RecordMovement)?;
    database.create_scoped_movement(input, &actor)
}

#[tauri::command]
pub fn create_movement_batch(
    input: CreateMovementBatchInput,
    database: State<'_, Database>,
    auth: State<'_, AuthService>,
) -> AppResult<MovementBatchView> {
    let actor = auth.require(&database, Permission::RecordMovement)?;
    database.create_movement_batch(input, &actor)
}

#[tauri::command]
pub fn create_asset_incident(
    input: CreateAssetIncidentInput,
    database: State<'_, Database>,
    auth: State<'_, AuthService>,
) -> AppResult<AssetIncidentView> {
    let actor = auth.require(&database, Permission::ReportAssetIncident)?;
    database.create_asset_incident(input, &actor)
}

#[tauri::command]
pub fn list_asset_incidents(
    database: State<'_, Database>,
    auth: State<'_, AuthService>,
) -> AppResult<Vec<AssetIncidentView>> {
    let actor = auth.require(&database, Permission::ReadInventory)?;
    database.list_asset_incidents(&actor)
}

#[tauri::command]
pub fn review_asset_incident(
    input: ReviewAssetIncidentInput,
    database: State<'_, Database>,
    auth: State<'_, AuthService>,
) -> AppResult<AssetIncidentView> {
    let actor = auth.require(&database, Permission::ReviewAssetIncident)?;
    database.review_asset_incident(input, &actor)
}

#[tauri::command]
pub fn create_backup(
    database: State<'_, Database>,
    auth: State<'_, AuthService>,
) -> AppResult<BackupRecord> {
    auth.require(&database, Permission::ManageOperations)?;
    database.create_backup()
}

#[tauri::command]
pub fn list_backups(
    database: State<'_, Database>,
    auth: State<'_, AuthService>,
) -> AppResult<Vec<BackupRecord>> {
    auth.require(&database, Permission::ManageOperations)?;
    database.list_backups()
}

#[tauri::command]
pub fn restore_backup(
    file_name: String,
    database: State<'_, Database>,
    auth: State<'_, AuthService>,
) -> AppResult<RestoreResult> {
    let actor = auth.require(&database, Permission::RestoreBackup)?;
    database.restore_backup(&file_name, &actor.id)
}

#[tauri::command]
pub fn recovery_key_status(
    database: State<'_, Database>,
    auth: State<'_, AuthService>,
) -> AppResult<RecoveryKeyStatus> {
    auth.require(&database, Permission::ManageEncryption)?;
    database.recovery_key_status()
}

#[tauri::command]
pub fn reveal_recovery_key(
    input: ConfirmSensitiveActionInput,
    database: State<'_, Database>,
    auth: State<'_, AuthService>,
) -> AppResult<RecoveryKeyReveal> {
    let actor = auth.reauthenticate(&database, Permission::ManageEncryption, input.password)?;
    Ok(RecoveryKeyReveal {
        key: database.reveal_recovery_key(&actor.id)?,
    })
}

#[tauri::command]
pub fn acknowledge_recovery_key(
    input: ConfirmSensitiveActionInput,
    database: State<'_, Database>,
    auth: State<'_, AuthService>,
) -> AppResult<RecoveryKeyStatus> {
    let actor = auth.reauthenticate(&database, Permission::ManageEncryption, input.password)?;
    database.acknowledge_recovery_key(&actor.id)
}

#[tauri::command]
pub fn preview_import(
    file_name: String,
    bytes: Vec<u8>,
    location_id: String,
    database: State<'_, Database>,
    imports: State<'_, ImportService>,
    auth: State<'_, AuthService>,
) -> AppResult<ImportPreview> {
    auth.require(&database, Permission::ManageOperations)?;
    let context = database.import_context(&location_id)?;
    imports.preview(&file_name, bytes, &location_id, context)
}

#[tauri::command]
pub fn confirm_import(
    token: String,
    database: State<'_, Database>,
    imports: State<'_, ImportService>,
    auth: State<'_, AuthService>,
) -> AppResult<ImportResult> {
    let actor = auth.require(&database, Permission::ManageOperations)?;
    let stage = imports.stage(&token)?;
    let result = database.import_stage(stage, &actor.id)?;
    imports.discard(&token);
    Ok(result)
}

#[tauri::command]
pub fn discard_import(
    token: String,
    database: State<'_, Database>,
    imports: State<'_, ImportService>,
    auth: State<'_, AuthService>,
) -> AppResult<()> {
    auth.require(&database, Permission::ManageOperations)?;
    imports.discard(&token);
    Ok(())
}

#[tauri::command]
pub fn create_adjustment_request(
    input: CreateAdjustmentRequestInput,
    database: State<'_, Database>,
    auth: State<'_, AuthService>,
) -> AppResult<AdjustmentRequestView> {
    let actor = auth.require(&database, Permission::RequestAdjustment)?;
    database.create_adjustment_request_for(input, &actor)
}

#[tauri::command]
pub fn list_adjustment_requests(
    database: State<'_, Database>,
    auth: State<'_, AuthService>,
) -> AppResult<Vec<AdjustmentRequestView>> {
    let actor = auth.require(&database, Permission::RequestAdjustment)?;
    let can_review = matches!(actor.role, UserRole::Manager | UserRole::Admin);
    database.list_adjustment_requests_for(&actor, can_review)
}

#[tauri::command]
pub fn review_adjustment_request(
    input: ReviewAdjustmentRequestInput,
    database: State<'_, Database>,
    auth: State<'_, AuthService>,
) -> AppResult<AdjustmentRequestView> {
    let actor = auth.require(&database, Permission::ReviewAdjustment)?;
    database.review_adjustment_request_for(input, &actor)
}

#[tauri::command]
pub fn create_replenishment_request(
    input: CreateReplenishmentRequestInput,
    database: State<'_, Database>,
    auth: State<'_, AuthService>,
) -> AppResult<ReplenishmentRequestView> {
    let actor = auth.require(&database, Permission::RequestReplenishment)?;
    database.create_replenishment_request(input, &actor)
}

#[tauri::command]
pub fn list_replenishment_requests(
    database: State<'_, Database>,
    auth: State<'_, AuthService>,
) -> AppResult<Vec<ReplenishmentRequestView>> {
    let actor = auth.require(&database, Permission::RequestReplenishment)?;
    let can_review = matches!(actor.role, UserRole::Manager | UserRole::Admin);
    database.list_replenishment_requests(&actor, can_review)
}

#[tauri::command]
pub fn review_replenishment_request(
    input: ReviewReplenishmentRequestInput,
    database: State<'_, Database>,
    auth: State<'_, AuthService>,
) -> AppResult<ReplenishmentRequestView> {
    let actor = auth.require(&database, Permission::ReviewReplenishment)?;
    database.review_replenishment_request(input, &actor)
}

#[tauri::command]
pub fn list_users(
    database: State<'_, Database>,
    auth: State<'_, AuthService>,
) -> AppResult<Vec<UserView>> {
    auth.require(&database, Permission::ManageUsers)?;
    database.list_users()
}

#[tauri::command]
pub fn create_user(
    input: CreateUserInput,
    database: State<'_, Database>,
    auth: State<'_, AuthService>,
) -> AppResult<UserView> {
    let actor = auth.require(&database, Permission::ManageUsers)?;
    auth.create_user(&database, input, &actor.id)
}

#[tauri::command]
pub fn update_user(
    input: UpdateUserInput,
    database: State<'_, Database>,
    auth: State<'_, AuthService>,
) -> AppResult<UserView> {
    let actor = auth.require(&database, Permission::ManageUsers)?;
    auth.update_user(&database, input, &actor.id)
}

#[tauri::command]
pub fn reset_user_password(
    input: ResetUserPasswordInput,
    database: State<'_, Database>,
    auth: State<'_, AuthService>,
) -> AppResult<UserView> {
    let actor = auth.require(&database, Permission::ManageUsers)?;
    auth.reset_user_password(&database, input, &actor.id)
}
