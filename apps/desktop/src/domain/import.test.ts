import { describe, expect, it } from "vitest";
import { canConfirmImport, importPreviewSchema } from "./import";

const valid = importPreviewSchema.parse({
  token: "123e4567-e89b-42d3-a456-426614174000",
  fileName: "estoque.xlsx",
  sourceMode: "legacy",
  totalRows: 2,
  validRows: 2,
  errorRows: 0,
  productsToCreate: 1,
  assetsToCreate: 2,
  totalQuantity: 2,
  generalErrors: [],
  rows: [],
});

describe("import preview", () => {
  it("libera somente uma prévia integralmente válida", () => {
    expect(canConfirmImport(valid)).toBe(true);
    expect(canConfirmImport({ ...valid, token: null, errorRows: 1 })).toBe(false);
    expect(canConfirmImport({ ...valid, token: null, generalErrors: ["Sem dados"] })).toBe(false);
  });
});
