import { describe, expect, it } from "vitest";
import { createSeedDashboard } from "../infrastructure/mock/seed";
import { createItemInputSchema, dashboardSchema } from "./inventory";

describe("inventory domain", () => {
  it("validates the dashboard contract", () => {
    expect(dashboardSchema.parse(createSeedDashboard()).totalItems).toBe(8);
  });

  it("rejects negative stock", () => {
    const result = createItemInputSchema.safeParse({
      sku: "NB-2",
      name: "Notebook",
      category: "Notebooks",
      trackingType: "quantity",
      serialNumberPolicy: "not_applicable",
      locationId: null,
      initialQuantity: -1,
      minimumQuantity: 0,
      assetTag: null,
      serialNumber: null,
    });
    expect(result.success).toBe(false);
  });

  it("rejects initial stock during catalog creation", () => {
    const result = createItemInputSchema.safeParse({
      sku: "NB-E14",
      name: "Notebook",
      category: "Notebooks",
      trackingType: "serialized",
      serialNumberPolicy: "required",
      locationId: "2d6c9f1f-dbf5-4321-8500-000000000001",
      initialQuantity: 1,
      minimumQuantity: 0,
      assetTag: null,
      serialNumber: null,
    });
    expect(result.success).toBe(false);
  });
});
