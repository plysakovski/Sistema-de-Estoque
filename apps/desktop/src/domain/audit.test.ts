import { describe, expect, it } from "vitest";
import { auditFiltersSchema, auditLogSchema, buildAuditChanges } from "./audit";

const log = auditLogSchema.parse({
  id: "c9911580-a0bf-40d4-a3f1-5ce65ec0028b",
  actor: "Administrador",
  action: "update",
  entityType: "asset",
  entityId: "7a6c9f1f-dbf5-4321-8500-000000000001",
  entityName: "NB-0014",
  beforeData: { status: "available", locationId: "central" },
  afterData: { status: "maintenance", locationId: "sul", note: "Revisão" },
  occurredAt: "2026-08-29T11:47:00-03:00",
});

describe("audit domain", () => {
  it("builds human-readable changes from before and after data", () => {
    expect(buildAuditChanges(log)).toEqual([
      { field: "Unidade", before: "central", after: "sul" },
      { field: "Observação", before: "—", after: "Revisão" },
      { field: "Estado", before: "Disponível", after: "Manutenção" },
    ]);
  });

  it("rejects an inverted date interval", () => {
    expect(auditFiltersSchema.safeParse({ search: "", entityType: null, action: null, dateFrom: "2026-08-30", dateTo: "2026-08-29" }).success).toBe(false);
  });
});
