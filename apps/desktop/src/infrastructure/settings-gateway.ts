import { invoke } from "@tauri-apps/api/core";
import { backupRecordSchema, recoveryKeyRevealSchema, recoveryKeyStatusSchema, restoreResultSchema, type BackupRecord, type ConfirmSensitiveActionInput, type RecoveryKeyReveal, type RecoveryKeyStatus, type RestoreBackupInput, type RestoreResult, type SettingsGateway } from "../domain/settings";

function isTauriRuntime(): boolean {
  return "__TAURI_INTERNALS__" in window;
}

class TauriSettingsGateway implements SettingsGateway {
  async listBackups(): Promise<BackupRecord[]> {
    return backupRecordSchema.array().parse(await invoke("list_backups"));
  }

  async createBackup(): Promise<BackupRecord> {
    return backupRecordSchema.parse(await invoke("create_backup"));
  }

  async restoreBackup(input: RestoreBackupInput): Promise<RestoreResult> {
    return restoreResultSchema.parse(await invoke("restore_backup", { fileName: input.fileName }));
  }

  async recoveryKeyStatus(): Promise<RecoveryKeyStatus> {
    return recoveryKeyStatusSchema.parse(await invoke("recovery_key_status"));
  }

  async revealRecoveryKey(input: ConfirmSensitiveActionInput): Promise<RecoveryKeyReveal> {
    return recoveryKeyRevealSchema.parse(await invoke("reveal_recovery_key", { input }));
  }

  async acknowledgeRecoveryKey(input: ConfirmSensitiveActionInput): Promise<RecoveryKeyStatus> {
    return recoveryKeyStatusSchema.parse(await invoke("acknowledge_recovery_key", { input }));
  }
}

class MockSettingsGateway implements SettingsGateway {
  private recoveryAcknowledged = false;
  private readonly backups: BackupRecord[] = [
    { fileName: "stockmanager-20260829-124500-000.db", createdAt: "2026-08-29T12:45:00Z", sizeBytes: 188_416, kind: "manual", valid: true },
    { fileName: "pre-restore-20260828-171200-000.db", createdAt: "2026-08-28T17:12:00Z", sizeBytes: 184_320, kind: "safety", valid: true },
  ];

  async listBackups(): Promise<BackupRecord[]> {
    return this.backups.map((backup) => ({ ...backup }));
  }

  async createBackup(): Promise<BackupRecord> {
    const now = new Date();
    const fileName = `stockmanager-${now.toISOString().replace(/[-:TZ.]/g, "").slice(0, 17)}.db`;
    const backup: BackupRecord = { fileName, createdAt: now.toISOString(), sizeBytes: 192_512, kind: "manual", valid: true };
    this.backups.unshift(backup);
    return { ...backup };
  }

  async restoreBackup(input: RestoreBackupInput): Promise<RestoreResult> {
    const source = this.backups.find((backup) => backup.fileName === input.fileName && backup.valid);
    if (!source) throw new Error("Backup não encontrado ou inválido");
    const now = new Date();
    const safetyBackup = `pre-restore-${now.toISOString().replace(/[-:TZ.]/g, "").slice(0, 17)}.db`;
    this.backups.unshift({ fileName: safetyBackup, createdAt: now.toISOString(), sizeBytes: 192_512, kind: "safety", valid: true });
    return { restoredFrom: source.fileName, safetyBackup, restoredAt: now.toISOString() };
  }

  async recoveryKeyStatus(): Promise<RecoveryKeyStatus> {
    return { acknowledged: this.recoveryAcknowledged };
  }

  async revealRecoveryKey(input: ConfirmSensitiveActionInput): Promise<RecoveryKeyReveal> {
    if (!input.password) throw new Error("Confirme sua senha");
    return { key: "a".repeat(64) };
  }

  async acknowledgeRecoveryKey(input: ConfirmSensitiveActionInput): Promise<RecoveryKeyStatus> {
    if (!input.password) throw new Error("Confirme sua senha");
    this.recoveryAcknowledged = true;
    return { acknowledged: true };
  }
}

export const settingsGateway: SettingsGateway = isTauriRuntime() ? new TauriSettingsGateway() : new MockSettingsGateway();
