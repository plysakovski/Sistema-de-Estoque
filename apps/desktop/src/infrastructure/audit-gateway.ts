import { invoke } from "@tauri-apps/api/core";
import { auditLogSchema, type AuditFilters, type AuditGateway, type AuditLog } from "../domain/audit";

function isTauriRuntime(): boolean {
  return "__TAURI_INTERNALS__" in window;
}

const mockLogs = auditLogSchema.array().parse([
  { id: "c9911580-a0bf-40d4-a3f1-5ce65ec0028b", actor: "Administrador", action: "update", entityType: "asset", entityId: "7a6c9f1f-dbf5-4321-8500-000000000001", entityName: "NB-0014", beforeData: { locationId: "Escritório Central", status: "available" }, afterData: { locationId: "Filial Sul", status: "maintenance", note: "Revisão preventiva" }, occurredAt: "2026-08-29T11:47:00-03:00" },
  { id: "c9911580-a0bf-40d4-a3f1-5ce65ec0028c", actor: "Administrador", action: "create", entityType: "category", entityId: "3d6c9f1f-dbf5-4321-8500-000000000006", entityName: "Periféricos", beforeData: null, afterData: { name: "Periféricos", code: "PERIFERICOS", active: true }, occurredAt: "2026-08-28T15:10:00-03:00" },
  { id: "c9911580-a0bf-40d4-a3f1-5ce65ec0028d", actor: "Administrador", action: "transfer", entityType: "asset", entityId: "7a6c9f1f-dbf5-4321-8500-000000000002", entityName: "NB-0015", beforeData: { locationId: "Filial Sul" }, afterData: { locationId: "Escritório Central" }, occurredAt: "2026-08-27T09:32:00-03:00" },
  { id: "c9911580-a0bf-40d4-a3f1-5ce65ec0028e", actor: "Administrador", action: "create", entityType: "product", entityId: "fca1cd57-e8fc-45d6-bdda-07632f951ec5", entityName: "Notebook Lenovo ThinkPad E14", beforeData: null, afterData: { sku: "NOTE-E14", name: "Notebook Lenovo ThinkPad E14", trackingType: "serialized", initialQuantity: 1 }, occurredAt: "2026-08-26T14:05:00-03:00" },
]);

class TauriAuditGateway implements AuditGateway {
  async listLogs(filters: AuditFilters): Promise<AuditLog[]> {
    return auditLogSchema.array().parse(await invoke("list_audit_logs", { filters }));
  }
}

class MockAuditGateway implements AuditGateway {
  async listLogs(filters: AuditFilters): Promise<AuditLog[]> {
    const search = filters.search.toLocaleLowerCase("pt-BR");
    return mockLogs.filter((log) => {
      const date = log.occurredAt.slice(0, 10);
      return (!search || [log.actor, log.action, log.entityType, log.entityId, log.entityName].some((value) => value.toLocaleLowerCase("pt-BR").includes(search)))
        && (!filters.entityType || log.entityType === filters.entityType)
        && (!filters.action || log.action === filters.action)
        && (!filters.dateFrom || date >= filters.dateFrom)
        && (!filters.dateTo || date <= filters.dateTo);
    });
  }
}

export const auditGateway: AuditGateway = isTauriRuntime() ? new TauriAuditGateway() : new MockAuditGateway();
