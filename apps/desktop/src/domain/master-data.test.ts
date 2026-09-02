import { describe, expect, it } from "vitest";
import { saveMasterDataInputSchema } from "./master-data";

describe("master data domain", () => {
  it("accepts a valid normalized identifier", () => {
    expect(saveMasterDataInputSchema.parse({ id: null, name: "Almoxarifado", code: "ALMOX_01" }).code).toBe("ALMOX_01");
  });

  it("rejects spaces and special characters in codes", () => {
    expect(saveMasterDataInputSchema.safeParse({ id: null, name: "Almoxarifado", code: "ALMOX 01!" }).success).toBe(false);
  });
});
