import { describe, expect, it } from "vitest";
import { assetRecordSchema, countAssetsByStatus, updateAssetInputSchema } from "./assets";

const asset = assetRecordSchema.parse({
  id: "7a6c9f1f-dbf5-4321-8500-000000000001",
  productId: "fca1cd57-e8fc-45d6-bdda-07632f951ec5",
  sku: "NOTE-E14",
  productName: "Notebook Lenovo ThinkPad E14",
  category: "Notebooks",
  assetTag: "NB-0014",
  serialNumber: "PF4ABC1",
  locationId: "2d6c9f1f-dbf5-4321-8500-000000000001",
  location: "Escritório Central",
  status: "available",
  receiptBatchId: null,
  updatedAt: "2025-05-24T09:42:00-03:00",
});

describe("asset domain", () => {
  it("summarizes records by status", () => {
    expect(countAssetsByStatus([asset, { ...asset, id: "7a6c9f1f-dbf5-4321-8500-000000000002", status: "maintenance" }])).toEqual({
      total: 2,
      available: 1,
      in_use: 0,
      maintenance: 1,
      disposed: 0,
    });
  });

  it("does not allow disposed as a direct edit", () => {
    expect(updateAssetInputSchema.safeParse({ id: asset.id, locationId: asset.locationId, status: "disposed", note: "" }).success).toBe(false);
  });
});
