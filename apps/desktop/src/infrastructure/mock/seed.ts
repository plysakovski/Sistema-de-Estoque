import type { Asset, Dashboard, InventoryItem, Movement } from "../../domain/inventory";
import { mockMasterDataRecords } from "./master-data";

export const seedItems: InventoryItem[] = [
  { id: "fca1cd57-e8fc-45d6-bdda-07632f951ec5", sku: "NOTE-E14", name: "Notebook Lenovo ThinkPad E14", category: "Notebooks", trackingType: "serialized", serialNumberPolicy: "required", quantity: 2, minimumQuantity: 0, locations: "Escritório Central (2)", active: true, updatedAt: "2025-05-24T09:42:00-03:00" },
  { id: "ea681329-cdb5-46b2-b991-d25b55c21f40", sku: "MON-DELL24", name: "Monitor Dell 24\"", category: "Monitores", trackingType: "quantity", serialNumberPolicy: "not_applicable", quantity: 1, minimumQuantity: 2, locations: "Filial Sul (1)", active: true, updatedAt: "2025-05-24T08:15:00-03:00" },
  { id: "3e4b1121-e05a-4838-b425-910376187a24", sku: "OPT-7010", name: "Desktop Dell OptiPlex 7010", category: "Computadores", trackingType: "serialized", serialNumberPolicy: "required", quantity: 1, minimumQuantity: 0, locations: "Escritório Central (1)", active: true, updatedAt: "2025-05-23T17:30:00-03:00" },
  { id: "9e5f6689-c287-4980-af21-73ff042b0399", sku: "MOUSE-MX3", name: "Mouse Logitech MX Master 3S", category: "Periféricos", trackingType: "quantity", serialNumberPolicy: "not_applicable", quantity: 4, minimumQuantity: 2, locations: "Filial Sul (4)", active: true, updatedAt: "2025-05-23T17:30:00-03:00" },
];

export const seedAssets: Asset[] = [
  { id: "7a6c9f1f-dbf5-4321-8500-000000000001", productId: "fca1cd57-e8fc-45d6-bdda-07632f951ec5", assetTag: "NB-0014", serialNumber: "PF4ABC1", locationId: "2d6c9f1f-dbf5-4321-8500-000000000001", location: "Escritório Central", status: "available", receiptBatchId: null, updatedAt: "2025-05-24T09:42:00-03:00" },
  { id: "7a6c9f1f-dbf5-4321-8500-000000000002", productId: "fca1cd57-e8fc-45d6-bdda-07632f951ec5", assetTag: "NB-0015", serialNumber: "PF4ABC2", locationId: "2d6c9f1f-dbf5-4321-8500-000000000001", location: "Escritório Central", status: "in_use", receiptBatchId: null, updatedAt: "2025-05-24T09:42:00-03:00" },
  { id: "7a6c9f1f-dbf5-4321-8500-000000000003", productId: "3e4b1121-e05a-4838-b425-910376187a24", assetTag: "PC-0009", serialNumber: "D7010-09", locationId: "2d6c9f1f-dbf5-4321-8500-000000000001", location: "Escritório Central", status: "maintenance", receiptBatchId: null, updatedAt: "2025-05-23T17:30:00-03:00" },
];

export const seedMovements: Movement[] = [
  { id: "15455bb6-0ac5-469e-871c-b836f4944c3e", occurredAt: "2025-05-24T09:42:00-03:00", kind: "entry", itemName: "Notebook Lenovo ThinkPad E14", assetTag: "NB-0014", location: "Escritório Central", category: "Notebooks", quantity: 1, actor: "admin", note: "Cadastro inicial" },
  { id: "08b044fc-6244-4400-b32c-e0a6e1bfef7b", occurredAt: "2025-05-24T08:15:00-03:00", kind: "transfer", itemName: "Monitor Dell 24\"", assetTag: null, location: "Depósito → Filial Sul", category: "Monitores", quantity: 1, actor: "admin", note: "Transferência entre unidades" },
];

export function createSeedDashboard(items = seedItems, movements = seedMovements, assets = seedAssets): Dashboard {
  return {
    totalItems: items.reduce((total, item) => total + item.quantity, 0),
    locationCount: mockMasterDataRecords.location.filter((record) => record.active).length,
    categoryCount: mockMasterDataRecords.category.filter((record) => record.active).length,
    attentionCount: items.filter((item) => item.trackingType === "quantity" && item.quantity < item.minimumQuantity).length,
    maintenanceCount: assets.filter((asset) => asset.status === "maintenance").length,
    pendingAdjustmentCount: 0,
    inventoryByLocation: [
      { location: "Escritório Central", quantity: 3 },
      { location: "Filial Sul", quantity: 5 },
      { location: "Depósito", quantity: 0 },
    ].filter((entry) => mockMasterDataRecords.location.some((record) => record.active && record.name === entry.location)),
    lowStock: items.filter((item) => item.trackingType === "quantity" && item.quantity < item.minimumQuantity),
    recentMovements: movements.slice(0, 10),
    updatedAt: new Date().toISOString(),
  };
}
