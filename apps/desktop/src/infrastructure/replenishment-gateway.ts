import { invoke } from "@tauri-apps/api/core";
import {
  replenishmentRequestSchema,
  transferShipmentSchema,
  type CreateReplenishmentRequestInput,
  type DispatchReplenishmentTransferInput,
  type ReceiveReplenishmentTransferInput,
  type ReplenishmentGateway,
  type ReplenishmentRequest,
  type ReplenishmentTransferShipment,
  type ReviewReplenishmentRequestInput,
} from "../domain/replenishment";
import { seedItems } from "./mock/seed";
import { mockMasterDataRecords } from "./mock/master-data";

function isTauriRuntime() { return "__TAURI_INTERNALS__" in window; }

class TauriReplenishmentGateway implements ReplenishmentGateway {
  async list() { return replenishmentRequestSchema.array().parse(await invoke("list_replenishment_requests")); }
  async create(input: CreateReplenishmentRequestInput) { return replenishmentRequestSchema.parse(await invoke("create_replenishment_request", { input })); }
  async review(input: ReviewReplenishmentRequestInput) { return replenishmentRequestSchema.parse(await invoke("review_replenishment_request", { input })); }
  async dispatchTransfer(input: DispatchReplenishmentTransferInput) { return transferShipmentSchema.parse(await invoke("dispatch_replenishment_transfer", { input })); }
  async receiveTransfer(input: ReceiveReplenishmentTransferInput) { return transferShipmentSchema.parse(await invoke("receive_replenishment_transfer", { input })); }
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
  transferShipments: [],
  items: [
    { id: "af29f216-0000-4000-8000-000000000011", productId: "3e4b1121-e05a-4838-b425-910376187a24", productName: "Desktop Dell OptiPlex 7010", sku: "OPT-7010", trackingType: "serialized", serialNumberPolicy: "required", requestedQuantity: 2, stockSnapshot: 0, approvedQuantity: 2, transferQuantity: 0, purchaseQuantity: 2, receivedQuantity: 0, transferReceivedQuantity: 0, transferInTransitQuantity: 0, transferRejectedQuantity: 0, sourceLocationId: null, sourceLocation: null, purchaseReference: "OC-DEMO-01", status: "approved", version: 2, reservedAssets: [] },
    { id: "af29f216-0000-4000-8000-000000000012", productId: "ea681329-cdb5-46b2-b991-d25b55c21f40", productName: "Monitor Dell 24\"", sku: "MON-DELL24", trackingType: "quantity", serialNumberPolicy: "not_applicable", requestedQuantity: 3, stockSnapshot: 1, approvedQuantity: 3, transferQuantity: 0, purchaseQuantity: 3, receivedQuantity: 0, transferReceivedQuantity: 0, transferInTransitQuantity: 0, transferRejectedQuantity: 0, sourceLocationId: null, sourceLocation: null, purchaseReference: "OC-DEMO-01", status: "approved", version: 2, reservedAssets: [] },
  ],
};

class MockReplenishmentGateway implements ReplenishmentGateway {
  private readonly requests = [structuredClone(demoRequest)];
  async list() { return structuredClone(this.requests); }
  async create(input: CreateReplenishmentRequestInput) {
    const request: ReplenishmentRequest = {
      ...structuredClone(demoRequest), id: crypto.randomUUID(), destinationLocationId: input.destinationLocationId,
      destinationLocation: mockMasterDataRecords.location.find((location) => location.id === input.destinationLocationId)?.name ?? "Unidade",
      priority: input.priority, justification: input.justification, status: "pending", reviewerId: null,
      reviewer: null, reviewNote: null, reviewedAt: null, version: 1,
      items: input.items.map((entry) => {
        const product = seedItems.find((item) => item.id === entry.productId);
        if (!product) throw new Error("Produto não encontrado");
        return { id: crypto.randomUUID(), productId: product.id, productName: product.name, sku: product.sku, trackingType: product.trackingType, serialNumberPolicy: product.serialNumberPolicy, requestedQuantity: entry.quantity, stockSnapshot: product.quantity, approvedQuantity: null, transferQuantity: 0, purchaseQuantity: 0, receivedQuantity: 0, transferReceivedQuantity: 0, transferInTransitQuantity: 0, transferRejectedQuantity: 0, sourceLocationId: null, sourceLocation: null, purchaseReference: null, status: "pending" as const, version: 1, reservedAssets: [] };
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
      return { ...item, approvedQuantity, transferQuantity: decision.transferQuantity, purchaseQuantity: decision.purchaseQuantity, sourceLocationId: decision.sourceLocationId, sourceLocation: decision.sourceLocationId ? mockMasterDataRecords.location.find((location) => location.id === decision.sourceLocationId)?.name ?? "Unidade de origem" : null, purchaseReference: decision.purchaseReference, status: approvedQuantity > 0 ? "approved" : "rejected", version: item.version + 1 };
    });
    request.status = request.items.some((item) => (item.approvedQuantity ?? 0) > 0) ? "approved" : "rejected";
    request.reviewNote = input.note;
    request.reviewer = "Gestor Demo";
    request.reviewedAt = new Date().toISOString();
    request.version += 1;
    return structuredClone(request);
  }
  async dispatchTransfer(input: DispatchReplenishmentTransferInput) {
    const request = this.requests.find((item) => item.id === input.requestId);
    if (!request) throw new Error("Solicitação não encontrada");
    const first = request.items.find((item) => item.id === input.items[0]?.requestItemId);
    if (!first?.sourceLocationId || !first.sourceLocation) throw new Error("Origem não definida");
    const shipmentItems: ReplenishmentTransferShipment["items"] = [];
    for (const line of input.items) {
      const item = request.items.find((candidate) => candidate.id === line.requestItemId)!;
      item.transferInTransitQuantity += line.quantity;
      if (item.trackingType === "serialized") {
        for (const assetId of line.assetIds) {
          const asset = item.reservedAssets.find((candidate) => candidate.id === assetId);
          shipmentItems.push({ id: crypto.randomUUID(), requestItemId: item.id, productId: item.productId, productName: item.productName, sku: item.sku, trackingType: item.trackingType, assetId, assetTag: asset?.assetTag ?? assetId.slice(0, 8), serialNumber: asset?.serialNumber ?? null, quantity: 1, receivedQuantity: 0, rejectedQuantity: 0, receiptNote: null });
        }
      } else {
        shipmentItems.push({ id: crypto.randomUUID(), requestItemId: item.id, productId: item.productId, productName: item.productName, sku: item.sku, trackingType: item.trackingType, assetId: null, assetTag: null, serialNumber: null, quantity: line.quantity, receivedQuantity: 0, rejectedQuantity: 0, receiptNote: null });
      }
    }
    const shipment: ReplenishmentTransferShipment = {
      id: crypto.randomUUID(), requestId: request.id, sourceLocationId: first.sourceLocationId,
      sourceLocation: first.sourceLocation, destinationLocationId: request.destinationLocationId,
      destinationLocation: request.destinationLocation, reference: input.reference, dispatchNote: input.note,
      receiptReference: null, receiptNote: null, status: "dispatched" as const, dispatcher: "Gestor Demo",
      receiver: null, dispatchedAt: new Date().toISOString(), receivedAt: null, version: 1,
      items: shipmentItems,
    };
    request.transferShipments.push(shipment);
    request.status = "in_fulfillment";
    return structuredClone(shipment);
  }
  async receiveTransfer(input: ReceiveReplenishmentTransferInput) {
    const request = this.requests.find((candidate) => candidate.transferShipments.some((shipment) => shipment.id === input.shipmentId));
    const shipment = request?.transferShipments.find((candidate) => candidate.id === input.shipmentId);
    if (!request || !shipment) throw new Error("Despacho não encontrado");
    for (const decision of input.items) {
      const line = shipment.items.find((candidate) => candidate.id === decision.shipmentItemId);
      if (!line) continue;
      line.receivedQuantity += decision.receivedQuantity;
      line.rejectedQuantity += decision.rejectedQuantity;
      line.receiptNote = decision.note || line.receiptNote;
      const item = request.items.find((candidate) => candidate.id === line.requestItemId)!;
      item.transferInTransitQuantity -= decision.receivedQuantity + decision.rejectedQuantity;
      item.transferReceivedQuantity += decision.receivedQuantity;
      item.transferRejectedQuantity += decision.rejectedQuantity;
      item.status = item.receivedQuantity === item.purchaseQuantity && item.transferReceivedQuantity === item.transferQuantity ? "fulfilled" : "in_fulfillment";
    }
    const remaining = shipment.items.reduce((sum, line) => sum + line.quantity - line.receivedQuantity - line.rejectedQuantity, 0);
    const rejected = shipment.items.reduce((sum, line) => sum + line.rejectedQuantity, 0);
    shipment.status = remaining > 0 ? "partially_received" : rejected > 0 ? "divergent" : "received";
    shipment.receiptReference = input.reference;
    shipment.receiptNote = input.note;
    shipment.receiver = "Operador Demo";
    shipment.receivedAt = new Date().toISOString();
    shipment.version += 1;
    if (request.items.filter((item) => (item.approvedQuantity ?? 0) > 0).every((item) => item.status === "fulfilled")) {
      request.status = "fulfilled";
      request.fulfilledAt = new Date().toISOString();
    }
    return structuredClone(shipment);
  }
}

export const replenishmentGateway: ReplenishmentGateway = isTauriRuntime()
  ? new TauriReplenishmentGateway()
  : new MockReplenishmentGateway();
