use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum UserRole {
    Operator,
    Manager,
    Admin,
}

impl UserRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Operator => "operator",
            Self::Manager => "manager",
            Self::Admin => "admin",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "operator" => Some(Self::Operator),
            "manager" => Some(Self::Manager),
            "admin" => Some(Self::Admin),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthenticatedUser {
    pub id: String,
    pub username: String,
    pub display_name: String,
    pub role: UserRole,
}

#[derive(Debug)]
pub(crate) struct AuthUserRecord {
    pub user: AuthenticatedUser,
    pub password_hash: String,
    pub failed_login_attempts: i64,
    pub locked_until: Option<String>,
    pub session_version: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapAdminInput {
    pub username: String,
    pub display_name: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginInput {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthStatus {
    pub setup_required: bool,
    pub user: Option<AuthenticatedUser>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserView {
    pub id: String,
    pub username: String,
    pub display_name: String,
    pub role: UserRole,
    pub active: bool,
    pub failed_login_attempts: i64,
    pub locked_until: Option<String>,
    pub last_login_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub location_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateUserInput {
    pub username: String,
    pub display_name: String,
    pub role: UserRole,
    pub password: String,
    #[serde(default)]
    pub location_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateUserInput {
    pub id: String,
    pub display_name: String,
    pub role: UserRole,
    pub active: bool,
    #[serde(default)]
    pub location_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResetUserPasswordInput {
    pub id: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InventoryItem {
    pub id: String,
    pub sku: String,
    pub name: String,
    pub category: String,
    pub tracking_type: String,
    pub serial_number_policy: String,
    pub quantity: i64,
    pub minimum_quantity: i64,
    pub locations: String,
    pub active: bool,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateItemInput {
    pub sku: String,
    pub name: String,
    pub category: String,
    pub tracking_type: String,
    pub serial_number_policy: String,
    pub minimum_quantity: i64,
    pub location_id: Option<String>,
    pub initial_quantity: i64,
    pub asset_tag: Option<String>,
    pub serial_number: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocationOption {
    pub id: String,
    pub name: String,
    pub code: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MasterDataRecord {
    pub id: String,
    pub name: String,
    pub code: String,
    pub active: bool,
    pub usage_count: i64,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveMasterDataInput {
    pub id: Option<String>,
    pub name: String,
    pub code: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetView {
    pub id: String,
    pub product_id: String,
    pub asset_tag: String,
    pub serial_number: Option<String>,
    pub location_id: String,
    pub location: String,
    pub status: String,
    pub receipt_batch_id: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetRecordView {
    pub id: String,
    pub product_id: String,
    pub sku: String,
    pub product_name: String,
    pub category: String,
    pub asset_tag: String,
    pub serial_number: Option<String>,
    pub location_id: String,
    pub location: String,
    pub status: String,
    pub receipt_batch_id: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetFilterInput {
    pub search: String,
    pub location_id: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAssetInput {
    pub id: String,
    pub location_id: String,
    pub status: String,
    pub note: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateMovementInput {
    pub product_id: String,
    pub kind: String,
    pub quantity: i64,
    pub from_location_id: Option<String>,
    pub to_location_id: Option<String>,
    pub asset_id: Option<String>,
    pub asset_tag: Option<String>,
    pub serial_number: Option<String>,
    pub asset_status: Option<String>,
    pub note: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateMovementBatchInput {
    pub kind: String,
    pub reference: String,
    pub note: String,
    pub replenishment_request_id: Option<String>,
    pub items: Vec<CreateMovementInput>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MovementBatchView {
    pub id: String,
    pub kind: String,
    pub reference: String,
    pub replenishment_request_id: Option<String>,
    pub occurred_at: String,
    pub movements: Vec<MovementView>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateAssetIncidentInput {
    pub asset_id: String,
    pub custodian_name: Option<String>,
    pub description: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewAssetIncidentInput {
    pub id: String,
    pub status: String,
    pub resolution_note: String,
    pub version: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetIncidentView {
    pub id: String,
    pub asset_id: String,
    pub asset_tag: String,
    pub serial_number: Option<String>,
    pub product_name: String,
    pub location: String,
    pub receipt_batch_id: Option<String>,
    pub reporter: String,
    pub custodian_name: Option<String>,
    pub description: String,
    pub status: String,
    pub reviewer: Option<String>,
    pub resolution_note: Option<String>,
    pub reported_at: String,
    pub reviewed_at: Option<String>,
    pub resolved_at: Option<String>,
    pub version: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MovementView {
    pub id: String,
    pub occurred_at: String,
    pub kind: String,
    pub item_name: String,
    pub asset_tag: Option<String>,
    pub location: String,
    pub category: String,
    pub quantity: i64,
    pub actor: String,
    pub note: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocationTotal {
    pub location: String,
    pub quantity: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Dashboard {
    pub total_items: i64,
    pub location_count: i64,
    pub category_count: i64,
    pub attention_count: i64,
    pub maintenance_count: i64,
    pub pending_adjustment_count: i64,
    pub inventory_by_location: Vec<LocationTotal>,
    pub low_stock: Vec<InventoryItem>,
    pub recent_movements: Vec<MovementView>,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateAdjustmentRequestInput {
    pub product_id: String,
    pub asset_id: Option<String>,
    pub kind: String,
    pub location_id: Option<String>,
    pub requested_quantity: Option<i64>,
    pub requested_status: Option<String>,
    pub requested_location_id: Option<String>,
    pub description: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewAdjustmentRequestInput {
    pub id: String,
    pub decision: String,
    pub note: String,
    pub version: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdjustmentRequestView {
    pub id: String,
    pub kind: String,
    pub status: String,
    pub product_id: String,
    pub product_name: String,
    pub sku: String,
    pub asset_id: Option<String>,
    pub asset_tag: Option<String>,
    pub location_id: Option<String>,
    pub location: Option<String>,
    pub requested_quantity: Option<i64>,
    pub requested_status: Option<String>,
    pub requested_location_id: Option<String>,
    pub requested_location: Option<String>,
    pub description: String,
    pub requester_id: String,
    pub requester: String,
    pub reviewer_id: Option<String>,
    pub reviewer: Option<String>,
    pub review_note: Option<String>,
    pub requested_at: String,
    pub reviewed_at: Option<String>,
    pub version: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateReplenishmentItemInput {
    pub product_id: String,
    pub quantity: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateReplenishmentRequestInput {
    pub destination_location_id: String,
    pub priority: String,
    pub justification: String,
    pub items: Vec<CreateReplenishmentItemInput>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplenishmentDecisionItemInput {
    pub item_id: String,
    pub transfer_quantity: i64,
    pub purchase_quantity: i64,
    pub source_location_id: Option<String>,
    pub purchase_reference: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewReplenishmentRequestInput {
    pub id: String,
    pub note: String,
    pub version: i64,
    pub items: Vec<ReplenishmentDecisionItemInput>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplenishmentRequestItemView {
    pub id: String,
    pub product_id: String,
    pub product_name: String,
    pub sku: String,
    pub tracking_type: String,
    pub serial_number_policy: String,
    pub requested_quantity: i64,
    pub stock_snapshot: i64,
    pub approved_quantity: Option<i64>,
    pub transfer_quantity: i64,
    pub purchase_quantity: i64,
    pub received_quantity: i64,
    pub source_location_id: Option<String>,
    pub source_location: Option<String>,
    pub purchase_reference: Option<String>,
    pub status: String,
    pub version: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplenishmentRequestView {
    pub id: String,
    pub destination_location_id: String,
    pub destination_location: String,
    pub priority: String,
    pub justification: String,
    pub status: String,
    pub requester_id: String,
    pub requester: String,
    pub reviewer_id: Option<String>,
    pub reviewer: Option<String>,
    pub review_note: Option<String>,
    pub requested_at: String,
    pub reviewed_at: Option<String>,
    pub fulfilled_at: Option<String>,
    pub version: i64,
    pub items: Vec<ReplenishmentRequestItemView>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditLogView {
    pub id: String,
    pub actor: String,
    pub action: String,
    pub entity_type: String,
    pub entity_id: String,
    pub entity_name: String,
    pub before_data: Option<serde_json::Value>,
    pub after_data: Option<serde_json::Value>,
    pub occurred_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditFilterInput {
    pub search: String,
    pub entity_type: Option<String>,
    pub action: Option<String>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupRecord {
    pub file_name: String,
    pub created_at: String,
    pub size_bytes: u64,
    pub kind: String,
    pub valid: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreResult {
    pub restored_from: String,
    pub safety_backup: String,
    pub restored_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryKeyStatus {
    pub acknowledged: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryKeyReveal {
    pub key: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfirmSensitiveActionInput {
    pub password: String,
}
