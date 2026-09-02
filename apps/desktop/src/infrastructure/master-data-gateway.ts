import { invoke } from "@tauri-apps/api/core";
import {
  masterDataRecordSchema,
  type MasterDataGateway,
  type MasterDataKind,
  type MasterDataRecord,
  type SaveMasterDataInput,
} from "../domain/master-data";
import { mockMasterDataRecords } from "./mock/master-data";

function isTauriRuntime(): boolean {
  return "__TAURI_INTERNALS__" in window;
}

const commandNames = {
  location: { list: "list_location_records", save: "save_location", active: "set_location_active" },
  category: { list: "list_categories", save: "save_category", active: "set_category_active" },
} as const;

class TauriMasterDataGateway implements MasterDataGateway {
  async list(kind: MasterDataKind): Promise<MasterDataRecord[]> {
    return masterDataRecordSchema.array().parse(await invoke(commandNames[kind].list));
  }

  async save(kind: MasterDataKind, input: SaveMasterDataInput): Promise<MasterDataRecord> {
    return masterDataRecordSchema.parse(await invoke(commandNames[kind].save, { input }));
  }

  async setActive(kind: MasterDataKind, id: string, active: boolean): Promise<MasterDataRecord> {
    return masterDataRecordSchema.parse(await invoke(commandNames[kind].active, { id, active }));
  }
}

class MockMasterDataGateway implements MasterDataGateway {
  async list(kind: MasterDataKind): Promise<MasterDataRecord[]> {
    return [...mockMasterDataRecords[kind]].sort((left, right) => Number(right.active) - Number(left.active) || left.name.localeCompare(right.name, "pt-BR"));
  }

  async save(kind: MasterDataKind, input: SaveMasterDataInput): Promise<MasterDataRecord> {
    const records = mockMasterDataRecords[kind];
    const duplicate = records.find((record) => record.id !== input.id && (record.name.toLocaleLowerCase("pt-BR") === input.name.toLocaleLowerCase("pt-BR") || record.code.toUpperCase() === input.code.toUpperCase()));
    if (duplicate) throw new Error("Já existe um cadastro com esse nome ou código");
    const existing = input.id ? records.find((record) => record.id === input.id) : undefined;
    if (input.id && !existing) throw new Error("Cadastro não encontrado");
    if (existing) {
      existing.name = input.name;
      existing.code = input.code.toUpperCase();
      existing.updatedAt = new Date().toISOString();
      return { ...existing };
    }
    const record: MasterDataRecord = { id: crypto.randomUUID(), name: input.name, code: input.code.toUpperCase(), active: true, usageCount: 0, updatedAt: new Date().toISOString() };
    records.push(record);
    return { ...record };
  }

  async setActive(kind: MasterDataKind, id: string, active: boolean): Promise<MasterDataRecord> {
    const record = mockMasterDataRecords[kind].find((entry) => entry.id === id);
    if (!record) throw new Error("Cadastro não encontrado");
    if (!active && record.usageCount > 0) throw new Error(kind === "location" ? "A unidade ainda possui estoque ou ativos" : "A categoria ainda possui produtos vinculados");
    record.active = active;
    record.updatedAt = new Date().toISOString();
    return { ...record };
  }
}

export const masterDataGateway: MasterDataGateway = isTauriRuntime()
  ? new TauriMasterDataGateway()
  : new MockMasterDataGateway();
