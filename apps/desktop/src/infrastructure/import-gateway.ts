import { invoke } from "@tauri-apps/api/core";
import {
  importPreviewSchema,
  importResultSchema,
  type ImportGateway,
  type ImportPreview,
  type ImportResult,
  type PreviewImportInput,
} from "../domain/import";

function isTauriRuntime(): boolean {
  return "__TAURI_INTERNALS__" in window;
}

class TauriImportGateway implements ImportGateway {
  async preview(input: PreviewImportInput): Promise<ImportPreview> {
    return importPreviewSchema.parse(await invoke("preview_import", input));
  }

  async confirm(token: string): Promise<ImportResult> {
    return importResultSchema.parse(await invoke("confirm_import", { token }));
  }

  async discard(token: string): Promise<void> {
    await invoke("discard_import", { token });
  }
}

class MockImportGateway implements ImportGateway {
  private previews = new Map<string, ImportPreview>();

  async preview(input: PreviewImportInput): Promise<ImportPreview> {
    if (input.bytes.length === 0) throw new Error("O arquivo está vazio");
    const token = crypto.randomUUID();
    const preview = importPreviewSchema.parse({
      token,
      fileName: input.fileName,
      sourceMode: input.fileName.toLowerCase().endsWith(".csv") ? "standard" : "legacy",
      totalRows: 3,
      validRows: 3,
      errorRows: 0,
      productsToCreate: 2,
      assetsToCreate: 2,
      totalQuantity: 14,
      generalErrors: [],
      rows: [
        { rowNumber: 3, sheet: "Computadores", sku: "LEG-COMPUTADORES-DELL-01", name: "Dell Optiplex 3000", category: "Computadores", trackingType: "serialized", quantity: 1, assetTag: "ERSA1111", errors: [] },
        { rowNumber: 4, sheet: "Computadores", sku: "LEG-COMPUTADORES-DELL-01", name: "Dell Optiplex 3000", category: "Computadores", trackingType: "serialized", quantity: 1, assetTag: "ERSA2222", errors: [] },
        { rowNumber: 2, sheet: "CSV", sku: "CABO-01", name: "Cabo HDMI", category: "Periféricos", trackingType: "quantity", quantity: 12, assetTag: null, errors: [] },
      ],
    });
    this.previews.set(token, preview);
    return preview;
  }

  async confirm(token: string): Promise<ImportResult> {
    const preview = this.previews.get(token);
    if (!preview) throw new Error("A prévia expirou. Analise o arquivo novamente");
    this.previews.delete(token);
    return {
      fileName: preview.fileName,
      productsCreated: preview.productsToCreate,
      assetsCreated: preview.assetsToCreate,
      quantityImported: preview.totalQuantity,
      safetyBackup: `pre-import-${Date.now()}.db`,
      importedAt: new Date().toISOString(),
    };
  }

  async discard(token: string): Promise<void> {
    this.previews.delete(token);
  }
}

export const importGateway: ImportGateway = isTauriRuntime() ? new TauriImportGateway() : new MockImportGateway();
