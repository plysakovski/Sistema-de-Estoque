import { z } from "zod";

export const incidentStatusSchema = z.enum(["pending", "internal_repair", "external_assistance", "warranty", "resolved", "disposed"]);
export type IncidentStatus = z.infer<typeof incidentStatusSchema>;
export const assetIncidentSchema = z.object({ id: z.string().uuid(), assetId: z.string().uuid(), assetTag: z.string(), serialNumber: z.string().nullable(), productName: z.string(), location: z.string(), receiptBatchId: z.string().uuid().nullable(), reporter: z.string(), custodianName: z.string().nullable(), description: z.string(), status: incidentStatusSchema, reviewer: z.string().nullable(), resolutionNote: z.string().nullable(), reportedAt: z.string(), reviewedAt: z.string().nullable(), resolvedAt: z.string().nullable(), version: z.number().int().positive() });
export type AssetIncident = z.infer<typeof assetIncidentSchema>;
export interface CreateAssetIncidentInput { assetId: string; custodianName: string | null; description: string; }
export interface ReviewAssetIncidentInput { id: string; status: IncidentStatus; resolutionNote: string; version: number; }
export interface AssetIncidentGateway { list(): Promise<AssetIncident[]>; create(input: CreateAssetIncidentInput): Promise<AssetIncident>; review(input: ReviewAssetIncidentInput): Promise<AssetIncident>; }
