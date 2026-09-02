import { invoke } from "@tauri-apps/api/core";
import {
  adjustmentRequestSchema,
  type AdjustmentGateway,
  type AdjustmentRequest,
  type CreateAdjustmentRequestInput,
  type ReviewAdjustmentRequestInput,
} from "../domain/adjustments";

function isTauriRuntime(): boolean {
  return "__TAURI_INTERNALS__" in window;
}

class TauriAdjustmentGateway implements AdjustmentGateway {
  async list() {
    return adjustmentRequestSchema.array().parse(await invoke("list_adjustment_requests"));
  }

  async create(input: CreateAdjustmentRequestInput) {
    return adjustmentRequestSchema.parse(await invoke("create_adjustment_request", { input }));
  }

  async review(input: ReviewAdjustmentRequestInput) {
    return adjustmentRequestSchema.parse(await invoke("review_adjustment_request", { input }));
  }
}

class MockAdjustmentGateway implements AdjustmentGateway {
  private readonly requests: AdjustmentRequest[] = [];

  async list() {
    return structuredClone(this.requests);
  }

  async create(input: CreateAdjustmentRequestInput) {
    const request = adjustmentRequestSchema.parse({
      id: crypto.randomUUID(), kind: input.kind, status: "pending", productId: input.productId,
      productName: "Produto da demonstração", sku: "DEMO", assetId: input.assetId, assetTag: null,
      locationId: input.locationId, location: null, requestedQuantity: input.requestedQuantity,
      requestedStatus: input.requestedStatus, requestedLocationId: input.requestedLocationId,
      requestedLocation: null, description: input.description,
      requesterId: "123e4567-e89b-42d3-a456-426614174000", requester: "Administrador da demonstração",
      reviewerId: null, reviewer: null, reviewNote: null, requestedAt: new Date().toISOString(),
      reviewedAt: null, version: 1,
    });
    this.requests.unshift(request);
    return structuredClone(request);
  }

  async review(input: ReviewAdjustmentRequestInput) {
    const request = this.requests.find((entry) => entry.id === input.id);
    if (!request) throw new Error("Solicitação não encontrada");
    request.status = input.decision;
    request.reviewerId = "123e4567-e89b-42d3-a456-426614174000";
    request.reviewer = "Administrador da demonstração";
    request.reviewNote = input.note;
    request.reviewedAt = new Date().toISOString();
    request.version += 1;
    return structuredClone(request);
  }
}

export const adjustmentGateway: AdjustmentGateway = isTauriRuntime()
  ? new TauriAdjustmentGateway()
  : new MockAdjustmentGateway();
