import { z } from "zod";
import { serialNumberPolicySchema, trackingTypeSchema } from "./inventory";

export const replenishmentStatusSchema = z.enum([
  "pending", "approved", "partially_approved", "rejected", "in_fulfillment", "fulfilled", "cancelled",
]);
export const replenishmentPrioritySchema = z.enum(["low", "normal", "high", "urgent"]);

export const replenishmentItemSchema = z.object({
  id: z.string().uuid(), productId: z.string().uuid(), productName: z.string(), sku: z.string(),
  trackingType: trackingTypeSchema, serialNumberPolicy: serialNumberPolicySchema,
  requestedQuantity: z.number().int().positive(),
  stockSnapshot: z.number().int().nonnegative(), approvedQuantity: z.number().int().nonnegative().nullable(),
  transferQuantity: z.number().int().nonnegative(), purchaseQuantity: z.number().int().nonnegative(),
  receivedQuantity: z.number().int().nonnegative(),
  sourceLocationId: z.string().uuid().nullable(), sourceLocation: z.string().nullable(),
  purchaseReference: z.string().nullable(), status: replenishmentStatusSchema, version: z.number().int().positive(),
});

export const replenishmentRequestSchema = z.object({
  id: z.string().uuid(), destinationLocationId: z.string().uuid(), destinationLocation: z.string(),
  priority: replenishmentPrioritySchema, justification: z.string(), status: replenishmentStatusSchema,
  requesterId: z.string().uuid(), requester: z.string(), reviewerId: z.string().uuid().nullable(),
  reviewer: z.string().nullable(), reviewNote: z.string().nullable(), requestedAt: z.string(),
  reviewedAt: z.string().nullable(), fulfilledAt: z.string().nullable(), version: z.number().int().positive(),
  items: z.array(replenishmentItemSchema),
});

export type ReplenishmentRequest = z.infer<typeof replenishmentRequestSchema>;
export type ReplenishmentPriority = z.infer<typeof replenishmentPrioritySchema>;
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
export interface ReplenishmentGateway {
  list(): Promise<ReplenishmentRequest[]>;
  create(input: CreateReplenishmentRequestInput): Promise<ReplenishmentRequest>;
  review(input: ReviewReplenishmentRequestInput): Promise<ReplenishmentRequest>;
}
