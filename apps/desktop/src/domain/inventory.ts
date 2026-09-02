import { z } from "zod";

export const trackingTypeSchema = z.enum(["quantity", "serialized"]);
export type TrackingType = z.infer<typeof trackingTypeSchema>;
export const serialNumberPolicySchema = z.enum(["required", "optional", "not_applicable"]);
export type SerialNumberPolicy = z.infer<typeof serialNumberPolicySchema>;

export const movementKindSchema = z.enum(["entry", "exit", "transfer", "adjustment"]);
export type MovementKind = z.infer<typeof movementKindSchema>;

export const assetStatusSchema = z.enum(["available", "in_use", "maintenance", "disposed"]);
export type AssetStatus = z.infer<typeof assetStatusSchema>;

export const inventoryItemSchema = z.object({
  id: z.string().uuid(),
  sku: z.string(),
  name: z.string(),
  category: z.string(),
  trackingType: trackingTypeSchema,
  serialNumberPolicy: serialNumberPolicySchema,
  quantity: z.number().int().nonnegative(),
  minimumQuantity: z.number().int().nonnegative(),
  locations: z.string(),
  active: z.boolean(),
  updatedAt: z.string(),
});
export type InventoryItem = z.infer<typeof inventoryItemSchema>;

export const locationSchema = z.object({ id: z.string().uuid(), name: z.string(), code: z.string() });
export type Location = z.infer<typeof locationSchema>;

export const assetSchema = z.object({
  id: z.string().uuid(),
  productId: z.string().uuid(),
  assetTag: z.string(),
  serialNumber: z.string().nullable(),
  locationId: z.string().uuid(),
  location: z.string(),
  status: assetStatusSchema,
  receiptBatchId: z.string().uuid().nullable(),
  updatedAt: z.string(),
});
export type Asset = z.infer<typeof assetSchema>;

export const movementSchema = z.object({
  id: z.string().uuid(),
  occurredAt: z.string(),
  kind: movementKindSchema,
  itemName: z.string(),
  assetTag: z.string().nullable(),
  location: z.string(),
  category: z.string(),
  quantity: z.number().int().positive(),
  actor: z.string(),
  note: z.string(),
});
export type Movement = z.infer<typeof movementSchema>;

export const dashboardSchema = z.object({
  totalItems: z.number().int().nonnegative(),
  locationCount: z.number().int().nonnegative(),
  categoryCount: z.number().int().nonnegative(),
  attentionCount: z.number().int().nonnegative(),
  maintenanceCount: z.number().int().nonnegative(),
  pendingAdjustmentCount: z.number().int().nonnegative(),
  inventoryByLocation: z.array(z.object({ location: z.string(), quantity: z.number().int().nonnegative() })),
  lowStock: z.array(inventoryItemSchema),
  recentMovements: z.array(movementSchema),
  updatedAt: z.string(),
});
export type Dashboard = z.infer<typeof dashboardSchema>;

export const createItemInputSchema = z
  .object({
    sku: z.string().trim().min(2).max(64),
    name: z.string().trim().min(2).max(160),
    category: z.string().trim().min(2).max(80),
    trackingType: trackingTypeSchema,
    serialNumberPolicy: serialNumberPolicySchema,
    minimumQuantity: z.number().int().nonnegative(),
    locationId: z.string().uuid().nullable(),
    initialQuantity: z.number().int().nonnegative(),
    assetTag: z.string().trim().max(64).nullable(),
    serialNumber: z.string().trim().max(120).nullable(),
  })
  .superRefine((value, context) => {
    if (value.initialQuantity !== 0 || value.locationId || value.assetTag || value.serialNumber) context.addIssue({ code: "custom", message: "O produto deve ser cadastrado sem saldo inicial", path: ["initialQuantity"] });
    if (value.trackingType === "quantity" && value.serialNumberPolicy !== "not_applicable") context.addIssue({ code: "custom", message: "Produtos por quantidade não usam número de série", path: ["serialNumberPolicy"] });
    if (value.trackingType === "serialized" && value.serialNumberPolicy === "not_applicable") context.addIssue({ code: "custom", message: "Defina a política de número de série", path: ["serialNumberPolicy"] });
  });
export type CreateItemInput = z.infer<typeof createItemInputSchema>;

export const createMovementInputSchema = z.object({
  productId: z.string().uuid(),
  kind: movementKindSchema,
  quantity: z.number().int().nonnegative(),
  fromLocationId: z.string().uuid().nullable(),
  toLocationId: z.string().uuid().nullable(),
  assetId: z.string().uuid().nullable(),
  assetTag: z.string().trim().max(64).nullable(),
  serialNumber: z.string().trim().max(120).nullable(),
  assetStatus: assetStatusSchema.nullable(),
  note: z.string().trim().max(500),
});
export type CreateMovementInput = z.infer<typeof createMovementInputSchema>;

export const movementBatchSchema = z.object({
  id: z.string().uuid(),
  kind: movementKindSchema,
  reference: z.string(),
  replenishmentRequestId: z.string().uuid().nullable(),
  occurredAt: z.string(),
  movements: z.array(movementSchema),
});
export type MovementBatch = z.infer<typeof movementBatchSchema>;
export interface CreateMovementBatchInput {
  kind: MovementKind;
  reference: string;
  note: string;
  replenishmentRequestId: string | null;
  items: CreateMovementInput[];
}

export interface InventoryGateway {
  getDashboard(): Promise<Dashboard>;
  listItems(search?: string): Promise<InventoryItem[]>;
  createItem(input: CreateItemInput): Promise<InventoryItem>;
  listLocations(): Promise<Location[]>;
  listAssets(productId: string): Promise<Asset[]>;
  listMovements(): Promise<Movement[]>;
  createMovement(input: CreateMovementInput): Promise<Movement>;
  createMovementBatch(input: CreateMovementBatchInput): Promise<MovementBatch>;
}
