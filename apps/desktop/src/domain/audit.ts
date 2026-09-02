import { z } from "zod";

const auditDataSchema = z.record(z.string(), z.unknown()).nullable();

export const auditLogSchema = z.object({
  id: z.string().uuid(),
  actor: z.string(),
  action: z.string(),
  entityType: z.string(),
  entityId: z.string(),
  entityName: z.string(),
  beforeData: auditDataSchema,
  afterData: auditDataSchema,
  occurredAt: z.string(),
});
export type AuditLog = z.infer<typeof auditLogSchema>;

export const auditFiltersSchema = z.object({
  search: z.string().trim().max(160),
  entityType: z.string().trim().max(40).nullable(),
  action: z.string().trim().max(40).nullable(),
  dateFrom: z.string().nullable(),
  dateTo: z.string().nullable(),
}).refine((filters) => !filters.dateFrom || !filters.dateTo || filters.dateFrom <= filters.dateTo, {
  message: "A data inicial não pode ser posterior à data final",
});
export type AuditFilters = z.infer<typeof auditFiltersSchema>;

export interface AuditGateway {
  listLogs(filters: AuditFilters): Promise<AuditLog[]>;
}

export interface AuditChange {
  field: string;
  before: string;
  after: string;
}

const fieldLabels: Record<string, string> = {
  active: "Situação",
  assetTag: "Patrimônio",
  code: "Código",
  initialQuantity: "Quantidade inicial",
  locationId: "Unidade",
  name: "Nome",
  note: "Observação",
  quantity: "Quantidade",
  sku: "SKU",
  status: "Estado",
  trackingType: "Tipo de controle",
};

function formatValue(value: unknown): string {
  if (value === undefined || value === null) return "—";
  if (typeof value === "boolean") return value ? "Sim" : "Não";
  if (typeof value === "object") return JSON.stringify(value);
  if (typeof value === "string") {
    return {
      available: "Disponível",
      in_use: "Em uso",
      maintenance: "Manutenção",
      disposed: "Baixado",
      quantity: "Quantidade",
      serialized: "Patrimonial",
    }[value] ?? value;
  }
  return String(value);
}

export function buildAuditChanges(log: AuditLog): AuditChange[] {
  const keys = new Set([...Object.keys(log.beforeData ?? {}), ...Object.keys(log.afterData ?? {})]);
  return [...keys].sort().map((key) => ({
    field: fieldLabels[key] ?? key,
    before: formatValue(log.beforeData?.[key]),
    after: formatValue(log.afterData?.[key]),
  }));
}
