import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { RestoreBackupInput } from "../../domain/settings";
import { settingsGateway } from "../../infrastructure/settings-gateway";

export function useBackups() {
  return useQuery({ queryKey: ["backups"], queryFn: () => settingsGateway.listBackups() });
}

export function useCreateBackup() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: () => settingsGateway.createBackup(),
    onSuccess: async () => client.invalidateQueries({ queryKey: ["backups"] }),
  });
}

export function useRestoreBackup() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: (input: RestoreBackupInput) => settingsGateway.restoreBackup(input),
    onSuccess: async () => client.invalidateQueries(),
  });
}

export function useRecoveryKeyStatus(enabled: boolean) {
  return useQuery({ queryKey: ["recovery-key-status"], queryFn: () => settingsGateway.recoveryKeyStatus(), enabled });
}

export function useRevealRecoveryKey() {
  return useMutation({ mutationFn: (password: string) => settingsGateway.revealRecoveryKey({ password }) });
}

export function useAcknowledgeRecoveryKey() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: (password: string) => settingsGateway.acknowledgeRecoveryKey({ password }),
    onSuccess: (status) => client.setQueryData(["recovery-key-status"], status),
  });
}
