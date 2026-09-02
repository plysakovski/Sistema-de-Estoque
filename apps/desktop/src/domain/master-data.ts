import { z } from "zod";

export const masterDataKindSchema = z.enum(["location", "category"]);
export type MasterDataKind = z.infer<typeof masterDataKindSchema>;

export const masterDataRecordSchema = z.object({
  id: z.string().uuid(),
  name: z.string(),
  code: z.string(),
  active: z.boolean(),
  usageCount: z.number().int().nonnegative(),
  updatedAt: z.string(),
});
export type MasterDataRecord = z.infer<typeof masterDataRecordSchema>;

export const saveMasterDataInputSchema = z.object({
  id: z.string().uuid().nullable(),
  name: z.string().trim().min(2, "O nome deve ter ao menos 2 caracteres").max(80),
  code: z
    .string()
    .trim()
    .min(2, "O código deve ter ao menos 2 caracteres")
    .max(32)
    .regex(/^[A-Za-z0-9_-]+$/, "Use apenas letras, números, hífen ou sublinhado"),
});
export type SaveMasterDataInput = z.infer<typeof saveMasterDataInputSchema>;

export interface MasterDataGateway {
  list(kind: MasterDataKind): Promise<MasterDataRecord[]>;
  save(kind: MasterDataKind, input: SaveMasterDataInput): Promise<MasterDataRecord>;
  setActive(kind: MasterDataKind, id: string, active: boolean): Promise<MasterDataRecord>;
}
