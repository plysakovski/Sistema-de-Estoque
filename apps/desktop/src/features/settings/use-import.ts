import { useMutation, useQueryClient } from "@tanstack/react-query";
import type { PreviewImportInput } from "../../domain/import";
import { importGateway } from "../../infrastructure/import-gateway";

export function usePreviewImport() {
  return useMutation({ mutationFn: (input: PreviewImportInput) => importGateway.preview(input) });
}

export function useConfirmImport() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: (token: string) => importGateway.confirm(token),
    onSuccess: async () => client.invalidateQueries(),
  });
}

export function discardImport(token: string) {
  return importGateway.discard(token);
}
