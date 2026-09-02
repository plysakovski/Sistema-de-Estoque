import { describe, expect, it } from "vitest";
import { backupRecordSchema, restoreBackupInputSchema } from "./settings";

describe("settings domain", () => {
  it("accepts a valid internal backup record", () => {
    expect(backupRecordSchema.safeParse({ fileName: "stockmanager-20260829-130000.db", createdAt: "2026-08-29T13:00:00Z", sizeBytes: 4096, kind: "manual", valid: true }).success).toBe(true);
  });

  it("rejects an empty restore target", () => {
    expect(restoreBackupInputSchema.safeParse({ fileName: "" }).success).toBe(false);
  });
});
