import { z } from "zod";
import { assetStatusSchema } from "./inventory";

export const editableAssetStatusSchema = z.enum(["available", "in_use", "maintenance"]);
export type EditableAssetStatus = z.infer<typeof editableAssetStatusSchema>;

export const assetRecordSchema = z.object({
  id: z.string().uuid(),
  productId: z.string().uuid(),
  sku: z.string(),
  productName: z.string(),
  category: z.string(),
  assetTag: z.string(),
  serialNumber: z.string().nullable(),
  locationId: z.string().uuid(),
  location: z.string(),
  status: assetStatusSchema,
  receiptBatchId: z.string().uuid().nullable(),
  updatedAt: z.string(),
});
export type AssetRecord = z.infer<typeof assetRecordSchema>;

export const assetFiltersSchema = z.object({
  search: z.string().trim().max(160),
  locationId: z.string().uuid().nullable(),
  status: assetStatusSchema.nullable(),
});
export type AssetFilters = z.infer<typeof assetFiltersSchema>;

export const updateAssetInputSchema = z.object({
  id: z.string().uuid(),
  locationId: z.string().uuid(),
  status: editableAssetStatusSchema,
  note: z.string().trim().max(500),
});
export type UpdateAssetInput = z.infer<typeof updateAssetInputSchema>;

export interface AssetGateway {
  listAssets(filters: AssetFilters): Promise<AssetRecord[]>;
  updateAsset(input: UpdateAssetInput): Promise<AssetRecord>;
}

export function countAssetsByStatus(records: AssetRecord[]) {
  return records.reduce(
    (summary, asset) => {
      summary.total += 1;
      summary[asset.status] += 1;
      return summary;
    },
    { total: 0, available: 0, in_use: 0, maintenance: 0, disposed: 0 },
  );
}
