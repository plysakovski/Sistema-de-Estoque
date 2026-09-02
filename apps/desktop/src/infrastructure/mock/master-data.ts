import type { MasterDataKind, MasterDataRecord } from "../../domain/master-data";

export const mockMasterDataRecords: Record<MasterDataKind, MasterDataRecord[]> = {
  location: [
    { id: "2d6c9f1f-dbf5-4321-8500-000000000001", name: "Escritório Central", code: "CENTRAL", active: true, usageCount: 3, updatedAt: "2025-05-24T10:15:00-03:00" },
    { id: "2d6c9f1f-dbf5-4321-8500-000000000002", name: "Filial Sul", code: "SUL", active: true, usageCount: 5, updatedAt: "2025-05-24T10:15:00-03:00" },
    { id: "2d6c9f1f-dbf5-4321-8500-000000000003", name: "Depósito", code: "DEPOSITO", active: true, usageCount: 0, updatedAt: "2025-05-24T10:15:00-03:00" },
  ],
  category: [
    { id: "3d6c9f1f-dbf5-4321-8500-000000000001", name: "Computadores", code: "COMPUTADORES", active: true, usageCount: 1, updatedAt: "2025-05-24T10:15:00-03:00" },
    { id: "3d6c9f1f-dbf5-4321-8500-000000000002", name: "Notebooks", code: "NOTEBOOKS", active: true, usageCount: 1, updatedAt: "2025-05-24T10:15:00-03:00" },
    { id: "3d6c9f1f-dbf5-4321-8500-000000000003", name: "Monitores", code: "MONITORES", active: true, usageCount: 1, updatedAt: "2025-05-24T10:15:00-03:00" },
    { id: "3d6c9f1f-dbf5-4321-8500-000000000006", name: "Periféricos", code: "PERIFERICOS", active: true, usageCount: 1, updatedAt: "2025-05-24T10:15:00-03:00" },
  ],
};
