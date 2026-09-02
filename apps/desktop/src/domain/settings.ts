import { z } from "zod";

export const backupRecordSchema = z.object({
  fileName: z.string().min(1),
  createdAt: z.string(),
  sizeBytes: z.number().int().nonnegative(),
  kind: z.enum(["manual", "safety", "import"]),
  valid: z.boolean(),
});
export type BackupRecord = z.infer<typeof backupRecordSchema>;

export const restoreResultSchema = z.object({
  restoredFrom: z.string(),
  safetyBackup: z.string(),
  restoredAt: z.string(),
});
export type RestoreResult = z.infer<typeof restoreResultSchema>;

export const restoreBackupInputSchema = z.object({ fileName: z.string().min(1).max(255) });
export type RestoreBackupInput = z.infer<typeof restoreBackupInputSchema>;

export const recoveryKeyStatusSchema = z.object({ acknowledged: z.boolean() });
export type RecoveryKeyStatus = z.infer<typeof recoveryKeyStatusSchema>;
export const recoveryKeyRevealSchema = z.object({ key: z.string().regex(/^[a-f0-9]{64}$/) });
export type RecoveryKeyReveal = z.infer<typeof recoveryKeyRevealSchema>;
export type ConfirmSensitiveActionInput = { password: string };

export interface SettingsGateway {
  listBackups(): Promise<BackupRecord[]>;
  createBackup(): Promise<BackupRecord>;
  restoreBackup(input: RestoreBackupInput): Promise<RestoreResult>;
  recoveryKeyStatus(): Promise<RecoveryKeyStatus>;
  revealRecoveryKey(input: ConfirmSensitiveActionInput): Promise<RecoveryKeyReveal>;
  acknowledgeRecoveryKey(input: ConfirmSensitiveActionInput): Promise<RecoveryKeyStatus>;
}
