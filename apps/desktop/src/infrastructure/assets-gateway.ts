import { invoke } from "@tauri-apps/api/core";
import {
  assetRecordSchema,
  type AssetFilters,
  type AssetGateway,
  type AssetRecord,
  type UpdateAssetInput,
} from "../domain/assets";
import { seedAssets, seedItems } from "./mock/seed";
import { mockMasterDataRecords } from "./mock/master-data";

function isTauriRuntime(): boolean {
  return "__TAURI_INTERNALS__" in window;
}

class TauriAssetGateway implements AssetGateway {
  async listAssets(filters: AssetFilters): Promise<AssetRecord[]> {
    return assetRecordSchema.array().parse(await invoke("list_asset_records", { filters }));
  }

  async updateAsset(input: UpdateAssetInput): Promise<AssetRecord> {
    return assetRecordSchema.parse(await invoke("update_asset", { input }));
  }
}

class MockAssetGateway implements AssetGateway {
  private readonly records: AssetRecord[] = seedAssets.map((asset) => {
    const product = seedItems.find((item) => item.id === asset.productId);
    if (!product) throw new Error(`Produto ausente para o ativo ${asset.assetTag}`);
    return { ...asset, sku: product.sku, productName: product.name, category: product.category };
  });

  async listAssets(filters: AssetFilters): Promise<AssetRecord[]> {
    const search = filters.search.toLocaleLowerCase("pt-BR");
    return this.records.filter((asset) => {
      const matchesSearch = !search || [asset.assetTag, asset.serialNumber ?? "", asset.sku, asset.productName, asset.category]
        .some((value) => value.toLocaleLowerCase("pt-BR").includes(search));
      return matchesSearch
        && (!filters.locationId || asset.locationId === filters.locationId)
        && (!filters.status || asset.status === filters.status);
    });
  }

  async updateAsset(input: UpdateAssetInput): Promise<AssetRecord> {
    const asset = this.records.find((record) => record.id === input.id);
    if (!asset) throw new Error("Ativo não encontrado");
    if (asset.status === "disposed") throw new Error("Um ativo baixado não pode ser alterado");
    const location = mockMasterDataRecords.location.find((record) => record.active && record.id === input.locationId);
    if (!location) throw new Error("Unidade não encontrada ou inativa");
    if (asset.locationId === input.locationId && asset.status === input.status) throw new Error("Nenhuma alteração foi informada");
    asset.locationId = location.id;
    asset.location = location.name;
    asset.status = input.status;
    asset.updatedAt = new Date().toISOString();
    return { ...asset };
  }
}

export const assetGateway: AssetGateway = isTauriRuntime()
  ? new TauriAssetGateway()
  : new MockAssetGateway();
