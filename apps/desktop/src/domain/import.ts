import { z } from "zod";
import { trackingTypeSchema } from "./inventory";

export const importPreviewRowSchema = z.object({
  rowNumber: z.number().int().positive(),
  sheet: z.string(),
  sku: z.string(),
  name: z.string(),
  category: z.string(),
  trackingType: trackingTypeSchema.or(z.string()),
  quantity: z.number().int(),
  assetTag: z.string().nullable(),
  errors: z.array(z.string()),
});
export type ImportPreviewRow = z.infer<typeof importPreviewRowSchema>;

export const importPreviewSchema = z.object({
  token: z.string().uuid().nullable(),
  fileName: z.string(),
  sourceMode: z.enum(["standard", "legacy", "mixed", "unknown"]),
  totalRows: z.number().int().nonnegative(),
  validRows: z.number().int().nonnegative(),
  errorRows: z.number().int().nonnegative(),
  productsToCreate: z.number().int().nonnegative(),
  assetsToCreate: z.number().int().nonnegative(),
  totalQuantity: z.number().int().nonnegative(),
  generalErrors: z.array(z.string()),
  rows: z.array(importPreviewRowSchema),
});
export type ImportPreview = z.infer<typeof importPreviewSchema>;

export const importResultSchema = z.object({
  fileName: z.string(),
  productsCreated: z.number().int().nonnegative(),
  assetsCreated: z.number().int().nonnegative(),
  quantityImported: z.number().int().nonnegative(),
  safetyBackup: z.string(),
  importedAt: z.string(),
});
export type ImportResult = z.infer<typeof importResultSchema>;

export type PreviewImportInput = { fileName: string; bytes: number[]; locationId: string };

export interface ImportGateway {
  preview(input: PreviewImportInput): Promise<ImportPreview>;
  confirm(token: string): Promise<ImportResult>;
  discard(token: string): Promise<void>;
}

export function canConfirmImport(preview: ImportPreview): boolean {
  return preview.token !== null && preview.errorRows === 0 && preview.generalErrors.length === 0 && preview.productsToCreate > 0;
}
