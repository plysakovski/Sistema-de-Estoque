import { invoke } from "@tauri-apps/api/core";
import {
  assetSchema,
  dashboardSchema,
  inventoryItemSchema,
  locationSchema,
  movementSchema,
  movementBatchSchema,
  type Asset,
  type CreateItemInput,
  type CreateMovementInput,
  type CreateMovementBatchInput,
  type Dashboard,
  type InventoryGateway,
  type InventoryItem,
  type Location,
  type Movement,
} from "../domain/inventory";
import { createSeedDashboard, seedAssets, seedItems, seedMovements } from "./mock/seed";
import { mockMasterDataRecords } from "./mock/master-data";

function isTauriRuntime(): boolean {
  return "__TAURI_INTERNALS__" in window;
}

class TauriInventoryGateway implements InventoryGateway {
  async getDashboard(): Promise<Dashboard> {
    return dashboardSchema.parse(await invoke("get_dashboard"));
  }

  async listItems(search = ""): Promise<InventoryItem[]> {
    return inventoryItemSchema.array().parse(await invoke("list_items", { search }));
  }

  async createItem(input: CreateItemInput): Promise<InventoryItem> {
    return inventoryItemSchema.parse(await invoke("create_item", { input }));
  }

  async listLocations(): Promise<Location[]> {
    return locationSchema.array().parse(await invoke("list_locations"));
  }

  async listAssets(productId: string): Promise<Asset[]> {
    return assetSchema.array().parse(await invoke("list_assets", { productId }));
  }

  async listMovements(): Promise<Movement[]> {
    return movementSchema.array().parse(await invoke("list_movements"));
  }

  async createMovement(input: CreateMovementInput): Promise<Movement> {
    return movementSchema.parse(await invoke("create_movement", { input }));
  }

  async createMovementBatch(input: CreateMovementBatchInput) {
    return movementBatchSchema.parse(await invoke("create_movement_batch", { input }));
  }
}

class MockInventoryGateway implements InventoryGateway {
  private readonly items = structuredClone(seedItems);
  private readonly assets = structuredClone(seedAssets);
  private readonly movements = structuredClone(seedMovements);

  async getDashboard(): Promise<Dashboard> {
    return createSeedDashboard(this.items, this.movements, this.assets);
  }

  async listItems(search = ""): Promise<InventoryItem[]> {
    const term = search.trim().toLocaleLowerCase("pt-BR");
    if (!term) return [...this.items];
    return this.items.filter((item) =>
      [item.sku, item.name, item.category, item.locations].some((value) =>
        value.toLocaleLowerCase("pt-BR").includes(term),
      ),
    );
  }

  async createItem(input: CreateItemInput): Promise<InventoryItem> {
    const location = mockMasterDataRecords.location.find((entry) => entry.active && entry.id === input.locationId);
    const item: InventoryItem = {
      id: crypto.randomUUID(),
      sku: input.sku,
      name: input.name,
      category: input.category,
      trackingType: input.trackingType,
      serialNumberPolicy: input.serialNumberPolicy,
      quantity: input.initialQuantity,
      minimumQuantity: input.minimumQuantity,
      locations: location && input.initialQuantity > 0 ? `${location.name} (${input.initialQuantity})` : "Sem saldo",
      active: true,
      updatedAt: new Date().toISOString(),
    };
    this.items.unshift(item);
    if (input.trackingType === "serialized" && input.initialQuantity === 1 && input.assetTag && location) {
      this.assets.push({ id: crypto.randomUUID(), productId: item.id, assetTag: input.assetTag, serialNumber: input.serialNumber, locationId: location.id, location: location.name, status: "available", receiptBatchId: null, updatedAt: item.updatedAt });
    }
    if (input.initialQuantity > 0 && location) {
      this.movements.unshift({ id: crypto.randomUUID(), occurredAt: item.updatedAt, kind: "entry", itemName: item.name, assetTag: input.trackingType === "serialized" ? input.assetTag : null, location: location.name, category: item.category, quantity: input.initialQuantity, actor: "admin", note: "Cadastro inicial" });
    }
    return item;
  }

  async listLocations(): Promise<Location[]> {
    return mockMasterDataRecords.location
      .filter((record) => record.active)
      .map(({ id, name, code }) => ({ id, name, code }));
  }

  async listAssets(productId: string): Promise<Asset[]> {
    return this.assets.filter((asset) => asset.productId === productId && asset.status !== "disposed");
  }

  async listMovements(): Promise<Movement[]> {
    return [...this.movements];
  }

  async createMovement(input: CreateMovementInput): Promise<Movement> {
    const item = this.items.find((entry) => entry.id === input.productId);
    if (!item) throw new Error("Produto não encontrado");
    const from = mockMasterDataRecords.location.find((entry) => entry.active && entry.id === input.fromLocationId);
    const to = mockMasterDataRecords.location.find((entry) => entry.active && entry.id === input.toLocationId);
    const asset = input.assetId ? this.assets.find((entry) => entry.id === input.assetId) : undefined;
    const originalAssetLocation = asset?.location;
    if (item.trackingType === "quantity") {
      if ((input.kind === "exit" || input.kind === "transfer") && item.quantity < input.quantity) {
        throw new Error(`Saldo insuficiente: disponível ${item.quantity}`);
      }
      if (input.kind === "entry") item.quantity += input.quantity;
      if (input.kind === "exit") item.quantity -= input.quantity;
      if (input.kind === "adjustment") item.quantity = input.quantity;
    } else if (input.kind === "entry" && input.assetTag && to) {
      this.assets.push({ id: crypto.randomUUID(), productId: item.id, assetTag: input.assetTag, serialNumber: input.serialNumber, locationId: to.id, location: to.name, status: "available", receiptBatchId: null, updatedAt: new Date().toISOString() });
      item.quantity += 1;
    } else if (asset) {
      if (input.kind === "transfer" && to) {
        asset.locationId = to.id;
        asset.location = to.name;
      }
      if (input.kind === "exit") {
        asset.status = "disposed";
        item.quantity -= 1;
      }
      if (input.kind === "adjustment" && input.assetStatus) asset.status = input.assetStatus;
    }
    if (item.trackingType === "serialized") {
      const activeAssets = this.assets.filter((entry) => entry.productId === item.id && entry.status !== "disposed");
      item.quantity = activeAssets.length;
      const totals = new Map<string, number>();
      for (const entry of activeAssets) totals.set(entry.location, (totals.get(entry.location) ?? 0) + 1);
      item.locations = [...totals].map(([location, quantity]) => `${location} (${quantity})`).join(", ") || "Sem ativos";
    }
    item.updatedAt = new Date().toISOString();
    const movement: Movement = {
      id: crypto.randomUUID(),
      occurredAt: item.updatedAt,
      kind: input.kind,
      itemName: item.name,
      assetTag: asset?.assetTag ?? input.assetTag ?? null,
      location: input.kind === "transfer" ? `${from?.name ?? originalAssetLocation ?? ""} → ${to?.name ?? ""}` : to?.name ?? from?.name ?? originalAssetLocation ?? "",
      category: item.category,
      quantity: item.trackingType === "serialized" ? 1 : Math.max(input.quantity, 1),
      actor: "admin",
      note: input.note,
    };
    this.movements.unshift(movement);
    return movement;
  }

  async createMovementBatch(input: CreateMovementBatchInput) {
    const movements: Movement[] = [];
    for (const item of input.items) movements.push(await this.createMovement(item));
    return { id: crypto.randomUUID(), kind: input.kind, reference: input.reference, replenishmentRequestId: input.replenishmentRequestId, occurredAt: new Date().toISOString(), movements };
  }
}

export const inventoryGateway: InventoryGateway = isTauriRuntime()
  ? new TauriInventoryGateway()
  : new MockInventoryGateway();
