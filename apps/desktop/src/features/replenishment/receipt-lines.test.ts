import { describe, expect, it } from "vitest";
import type { ReplenishmentRequest } from "../../domain/replenishment";
import { buildReceiptLineSeeds } from "./receipt-lines";

const request = {
  id: "00000000-0000-4000-8000-000000000001",
  destinationLocationId: "00000000-0000-4000-8000-000000000002",
  destinationLocation: "Depósito",
  priority: "normal",
  justification: "Reposição necessária para a unidade",
  status: "in_fulfillment",
  requesterId: "00000000-0000-4000-8000-000000000003",
  requester: "Operador",
  reviewerId: "00000000-0000-4000-8000-000000000004",
  reviewer: "Gestor",
  reviewNote: "Compra aprovada para reposição",
  requestedAt: "2026-09-01T00:00:00Z",
  reviewedAt: "2026-09-01T01:00:00Z",
  fulfilledAt: null,
  version: 2,
  items: [
    {
      id: "00000000-0000-4000-8000-000000000005",
      productId: "00000000-0000-4000-8000-000000000006",
      productName: "Notebook X",
      sku: "NOTE-X",
      trackingType: "serialized",
      serialNumberPolicy: "required",
      requestedQuantity: 3,
      stockSnapshot: 0,
      approvedQuantity: 3,
      transferQuantity: 0,
      purchaseQuantity: 3,
      receivedQuantity: 1,
      sourceLocationId: null,
      sourceLocation: null,
      purchaseReference: "OC-10",
      status: "in_fulfillment",
      version: 2,
    },
    {
      id: "00000000-0000-4000-8000-000000000007",
      productId: "00000000-0000-4000-8000-000000000008",
      productName: "Mouse USB",
      sku: "MOUSE-USB",
      trackingType: "quantity",
      serialNumberPolicy: "not_applicable",
      requestedQuantity: 5,
      stockSnapshot: 0,
      approvedQuantity: 5,
      transferQuantity: 0,
      purchaseQuantity: 5,
      receivedQuantity: 2,
      sourceLocationId: null,
      sourceLocation: null,
      purchaseReference: "OC-10",
      status: "in_fulfillment",
      version: 2,
    },
  ],
} satisfies ReplenishmentRequest;

describe("buildReceiptLineSeeds", () => {
  it("creates one locked line for every pending serialized asset", () => {
    const lines = buildReceiptLineSeeds(request).filter((line) => line.productId.endsWith("6"));
    expect(lines).toHaveLength(2);
    expect(lines.map((line) => line.quantity)).toEqual(["1", "1"]);
    expect(lines[0]?.serialNumberPolicy).toBe("required");
  });

  it("creates a quantity line limited to the remaining approved purchase", () => {
    const line = buildReceiptLineSeeds(request).find((item) => item.productId.endsWith("8"));
    expect(line).toMatchObject({ quantity: "3", maxQuantity: 3, productName: "Mouse USB" });
  });
});
