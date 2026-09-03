import { z } from "zod";
import { serialNumberPolicySchema, trackingTypeSchema } from "./inventory";

export const replenishmentStatusSchema = z.enum([
  "pending", "approved", "partially_approved", "rejected", "in_fulfillment", "fulfilled", "cancelled",
]);
export const replenishmentPrioritySchema = z.enum(["low", "normal", "high", "urgent"]);

export const reservedTransferAssetSchema = z.object({
  id: z.string().uuid(), assetTag: z.string(), serialNumber: z.string().nullable(),
});

export const transferShipmentItemSchema = z.object({
  id: z.string().uuid(), requestItemId: z.string().uuid(), productId: z.string().uuid(),
  productName: z.string(), sku: z.string(), trackingType: trackingTypeSchema,
  assetId: z.string().uuid().nullable(), assetTag: z.string().nullable(), serialNumber: z.string().nullable(),
  quantity: z.number().int().positive(), receivedQuantity: z.number().int().nonnegative(),
  rejectedQuantity: z.number().int().nonnegative(), receiptNote: z.string().nullable(),
});

export const transferShipmentSchema = z.object({
  id: z.string().uuid(), requestId: z.string().uuid(), sourceLocationId: z.string().uuid(),
  sourceLocation: z.string(), destinationLocationId: z.string().uuid(), destinationLocation: z.string(),
  reference: z.string(), dispatchNote: z.string(), receiptReference: z.string().nullable(),
  receiptNote: z.string().nullable(), status: z.enum(["dispatched", "partially_received", "received", "divergent"]),
  dispatcher: z.string(), receiver: z.string().nullable(), dispatchedAt: z.string(),
  receivedAt: z.string().nullable(), version: z.number().int().positive(), items: z.array(transferShipmentItemSchema),
});

export const replenishmentItemSchema = z.object({
  id: z.string().uuid(), productId: z.string().uuid(), productName: z.string(), sku: z.string(),
  trackingType: trackingTypeSchema, serialNumberPolicy: serialNumberPolicySchema,
  requestedQuantity: z.number().int().positive(),
  stockSnapshot: z.number().int().nonnegative(), approvedQuantity: z.number().int().nonnegative().nullable(),
  transferQuantity: z.number().int().nonnegative(), purchaseQuantity: z.number().int().nonnegative(),
  receivedQuantity: z.number().int().nonnegative(),
  transferReceivedQuantity: z.number().int().nonnegative(),
  transferInTransitQuantity: z.number().int().nonnegative(),
  transferRejectedQuantity: z.number().int().nonnegative(),
  sourceLocationId: z.string().uuid().nullable(), sourceLocation: z.string().nullable(),
  purchaseReference: z.string().nullable(), status: replenishmentStatusSchema, version: z.number().int().positive(),
  reservedAssets: z.array(reservedTransferAssetSchema),
});

export const replenishmentRequestSchema = z.object({
  id: z.string().uuid(), destinationLocationId: z.string().uuid(), destinationLocation: z.string(),
  priority: replenishmentPrioritySchema, justification: z.string(), status: replenishmentStatusSchema,
  requesterId: z.string().uuid(), requester: z.string(), reviewerId: z.string().uuid().nullable(),
  reviewer: z.string().nullable(), reviewNote: z.string().nullable(), requestedAt: z.string(),
  reviewedAt: z.string().nullable(), fulfilledAt: z.string().nullable(), version: z.number().int().positive(),
  items: z.array(replenishmentItemSchema), transferShipments: z.array(transferShipmentSchema),
});

export type ReplenishmentRequest = z.infer<typeof replenishmentRequestSchema>;
export type ReplenishmentPriority = z.infer<typeof replenishmentPrioritySchema>;
export type ReplenishmentTransferShipment = z.infer<typeof transferShipmentSchema>;
export interface CreateReplenishmentRequestInput {
  destinationLocationId: string;
  priority: ReplenishmentPriority;
  justification: string;
  items: Array<{ productId: string; quantity: number }>;
}
export interface ReviewReplenishmentRequestInput {
  id: string;
  note: string;
  version: number;
  items: Array<{
    itemId: string;
    transferQuantity: number;
    purchaseQuantity: number;
    sourceLocationId: string | null;
    purchaseReference: string | null;
  }>;
}
export interface DispatchReplenishmentTransferInput {
  requestId: string;
  reference: string;
  note: string;
  items: Array<{ requestItemId: string; quantity: number; assetIds: string[] }>;
}
export interface ReceiveReplenishmentTransferInput {
  shipmentId: string;
  reference: string;
  note: string;
  version: number;
  items: Array<{ shipmentItemId: string; receivedQuantity: number; rejectedQuantity: number; note: string }>;
}
export interface ReplenishmentGateway {
  list(): Promise<ReplenishmentRequest[]>;
  create(input: CreateReplenishmentRequestInput): Promise<ReplenishmentRequest>;
  review(input: ReviewReplenishmentRequestInput): Promise<ReplenishmentRequest>;
  dispatchTransfer(input: DispatchReplenishmentTransferInput): Promise<ReplenishmentTransferShipment>;
  receiveTransfer(input: ReceiveReplenishmentTransferInput): Promise<ReplenishmentTransferShipment>;
}
