import { z } from "zod";

export const adjustmentStatusSchema = z.enum(["pending", "approved", "rejected", "cancelled"]);
export const adjustmentKindSchema = z.enum(["quantity_adjustment", "asset_update"]);

export const adjustmentRequestSchema = z.object({
  id: z.string().uuid(),
  kind: adjustmentKindSchema,
  status: adjustmentStatusSchema,
  productId: z.string().uuid(),
  productName: z.string(),
  sku: z.string(),
  assetId: z.string().uuid().nullable(),
  assetTag: z.string().nullable(),
  locationId: z.string().uuid().nullable(),
  location: z.string().nullable(),
  requestedQuantity: z.number().int().nonnegative().nullable(),
  requestedStatus: z.string().nullable(),
  requestedLocationId: z.string().uuid().nullable(),
  requestedLocation: z.string().nullable(),
  description: z.string(),
  requesterId: z.string().uuid(),
  requester: z.string(),
  reviewerId: z.string().uuid().nullable(),
  reviewer: z.string().nullable(),
  reviewNote: z.string().nullable(),
  requestedAt: z.string(),
  reviewedAt: z.string().nullable(),
  version: z.number().int().positive(),
});
export type AdjustmentRequest = z.infer<typeof adjustmentRequestSchema>;

export interface CreateAdjustmentRequestInput {
  productId: string;
  assetId: string | null;
  kind: z.infer<typeof adjustmentKindSchema>;
  locationId: string | null;
  requestedQuantity: number | null;
  requestedStatus: string | null;
  requestedLocationId: string | null;
  description: string;
}

export interface ReviewAdjustmentRequestInput {
  id: string;
  decision: "approved" | "rejected";
  note: string;
  version: number;
}

export interface AdjustmentGateway {
  list(): Promise<AdjustmentRequest[]>;
  create(input: CreateAdjustmentRequestInput): Promise<AdjustmentRequest>;
  review(input: ReviewAdjustmentRequestInput): Promise<AdjustmentRequest>;
}
