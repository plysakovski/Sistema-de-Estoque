import { invoke } from "@tauri-apps/api/core";
import {
  replenishmentRequestSchema,
  type CreateReplenishmentRequestInput,
  type ReplenishmentGateway,
  type ReplenishmentRequest,
  type ReviewReplenishmentRequestInput,
} from "../domain/replenishment";
import { seedItems } from "./mock/seed";

function isTauriRuntime() { return "__TAURI_INTERNALS__" in window; }

class TauriReplenishmentGateway implements ReplenishmentGateway {
  async list() { return replenishmentRequestSchema.array().parse(await invoke("list_replenishment_requests")); }
  async create(input: CreateReplenishmentRequestInput) { return replenishmentRequestSchema.parse(await invoke("create_replenishment_request", { input })); }
  async review(input: ReviewReplenishmentRequestInput) { return replenishmentRequestSchema.parse(await invoke("review_replenishment_request", { input })); }
}

const demoRequest: ReplenishmentRequest = {
  id: "af29f216-0000-4000-8000-000000000001",
  destinationLocationId: "2d6c9f1f-dbf5-4321-8500-000000000003",
  destinationLocation: "Depósito",
  priority: "urgent",
  justification: "Equipamentos necessários para recompor o estoque da unidade.",
  status: "approved",
  requesterId: "00000000-0000-4000-8000-000000000101",
  requester: "Arthur",
  reviewerId: "00000000-0000-4000-8000-000000000102",
  reviewer: "Gestor Demo",
  reviewNote: "Compra aprovada após análise da necessidade.",
  requestedAt: "2026-09-01T00:18:00-03:00",
  reviewedAt: "2026-09-01T00:25:00-03:00",
  fulfilledAt: null,
  version: 2,
  items: [
    { id: "af29f216-0000-4000-8000-000000000011", productId: "3e4b1121-e05a-4838-b425-910376187a24", productName: "Desktop Dell OptiPlex 7010", sku: "OPT-7010", trackingType: "serialized", serialNumberPolicy: "required", requestedQuantity: 2, stockSnapshot: 0, approvedQuantity: 2, transferQuantity: 0, purchaseQuantity: 2, receivedQuantity: 0, sourceLocationId: null, sourceLocation: null, purchaseReference: "OC-DEMO-01", status: "approved", version: 2 },
    { id: "af29f216-0000-4000-8000-000000000012", productId: "ea681329-cdb5-46b2-b991-d25b55c21f40", productName: "Monitor Dell 24\"", sku: "MON-DELL24", trackingType: "quantity", serialNumberPolicy: "not_applicable", requestedQuantity: 3, stockSnapshot: 1, approvedQuantity: 3, transferQuantity: 0, purchaseQuantity: 3, receivedQuantity: 0, sourceLocationId: null, sourceLocation: null, purchaseReference: "OC-DEMO-01", status: "approved", version: 2 },
  ],
};

class MockReplenishmentGateway implements ReplenishmentGateway {
  private readonly requests = [structuredClone(demoRequest)];
  async list() { return structuredClone(this.requests); }
  async create(input: CreateReplenishmentRequestInput) {
    const request: ReplenishmentRequest = {
      ...structuredClone(demoRequest), id: crypto.randomUUID(), destinationLocationId: input.destinationLocationId,
      priority: input.priority, justification: input.justification, status: "pending", reviewerId: null,
      reviewer: null, reviewNote: null, reviewedAt: null, version: 1,
      items: input.items.map((entry) => {
        const product = seedItems.find((item) => item.id === entry.productId);
        if (!product) throw new Error("Produto não encontrado");
        return { id: crypto.randomUUID(), productId: product.id, productName: product.name, sku: product.sku, trackingType: product.trackingType, serialNumberPolicy: product.serialNumberPolicy, requestedQuantity: entry.quantity, stockSnapshot: product.quantity, approvedQuantity: null, transferQuantity: 0, purchaseQuantity: 0, receivedQuantity: 0, sourceLocationId: null, sourceLocation: null, purchaseReference: null, status: "pending" as const, version: 1 };
      }),
    };
    this.requests.unshift(request);
    return structuredClone(request);
  }
  async review(input: ReviewReplenishmentRequestInput) {
    const request = this.requests.find((item) => item.id === input.id);
    if (!request) throw new Error("Solicitação não encontrada");
    request.items = request.items.map((item) => {
      const decision = input.items.find((entry) => entry.itemId === item.id);
      if (!decision) return item;
      const approvedQuantity = decision.transferQuantity + decision.purchaseQuantity;
      return { ...item, approvedQuantity, transferQuantity: decision.transferQuantity, purchaseQuantity: decision.purchaseQuantity, sourceLocationId: decision.sourceLocationId, purchaseReference: decision.purchaseReference, status: approvedQuantity > 0 ? "approved" : "rejected", version: item.version + 1 };
    });
    request.status = request.items.some((item) => (item.approvedQuantity ?? 0) > 0) ? "approved" : "rejected";
    request.reviewNote = input.note;
    request.reviewer = "Gestor Demo";
    request.reviewedAt = new Date().toISOString();
    request.version += 1;
    return structuredClone(request);
  }
}

export const replenishmentGateway: ReplenishmentGateway = isTauriRuntime()
  ? new TauriReplenishmentGateway()
  : new MockReplenishmentGateway();
