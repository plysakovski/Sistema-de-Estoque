import { invoke } from "@tauri-apps/api/core";
import { assetIncidentSchema, type AssetIncident, type AssetIncidentGateway, type CreateAssetIncidentInput, type ReviewAssetIncidentInput } from "../domain/asset-incidents";

function isTauriRuntime() { return "__TAURI_INTERNALS__" in window; }
class TauriAssetIncidentGateway implements AssetIncidentGateway {
  async list() { return assetIncidentSchema.array().parse(await invoke("list_asset_incidents")); }
  async create(input: CreateAssetIncidentInput) { return assetIncidentSchema.parse(await invoke("create_asset_incident", { input })); }
  async review(input: ReviewAssetIncidentInput) { return assetIncidentSchema.parse(await invoke("review_asset_incident", { input })); }
}
class MockAssetIncidentGateway implements AssetIncidentGateway {
  private incidents: AssetIncident[] = [];
  async list() { return [...this.incidents]; }
  async create(input: CreateAssetIncidentInput) { const incident = assetIncidentSchema.parse({ id: crypto.randomUUID(), assetId: input.assetId, assetTag: "NB-0014", serialNumber: "PF4ABC1", productName: "Notebook Lenovo ThinkPad E14", location: "Escritório Central", receiptBatchId: null, reporter: "Usuário da demonstração", custodianName: input.custodianName, description: input.description, status: "pending", reviewer: null, resolutionNote: null, reportedAt: new Date().toISOString(), reviewedAt: null, resolvedAt: null, version: 1 }); this.incidents.unshift(incident); return incident; }
  async review(input: ReviewAssetIncidentInput) { const current = this.incidents.find((item) => item.id === input.id); if (!current) throw new Error("Ocorrência não encontrada"); Object.assign(current, { status: input.status, resolutionNote: input.resolutionNote, reviewer: "Gestor da demonstração", reviewedAt: new Date().toISOString(), version: current.version + 1 }); return current; }
}
export const assetIncidentGateway: AssetIncidentGateway = isTauriRuntime() ? new TauriAssetIncidentGateway() : new MockAssetIncidentGateway();
