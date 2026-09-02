import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { AssetRecord } from "../../domain/assets";
import type { Location } from "../../domain/inventory";
import { EditAssetDialog } from "./edit-asset-dialog";

const deposit: Location = {
  id: "123e4567-e89b-42d3-a456-426614174001",
  name: "Depósito",
  code: "DEP",
};

const asset: AssetRecord = {
  id: "123e4567-e89b-42d3-a456-426614174002",
  productId: "123e4567-e89b-42d3-a456-426614174003",
  sku: "MOUSE-001",
  productName: "Mouse USB",
  category: "Periféricos",
  assetTag: "PAT-0001",
  serialNumber: "SER-0001",
  locationId: deposit.id,
  location: deposit.name,
  status: "available",
  receiptBatchId: null,
  updatedAt: "2026-08-31T12:00:00Z",
};

function renderDialog(operatorMode: boolean) {
  const queryClient = new QueryClient({ defaultOptions: { mutations: { retry: false } } });
  return render(
    <QueryClientProvider client={queryClient}>
      <EditAssetDialog asset={asset} locations={[deposit]} operatorMode={operatorMode} onClose={vi.fn()} />
    </QueryClientProvider>,
  );
}

describe("EditAssetDialog", () => {
  it("limita o operador a mudanças operacionais dentro da própria unidade", () => {
    renderDialog(true);

    const location = screen.getByLabelText("Unidade") as HTMLSelectElement;
    const status = screen.getByLabelText("Estado") as HTMLSelectElement;

    expect(location.disabled).toBe(true);
    expect(status.disabled).toBe(false);
    expect(Array.from(status.options, (option) => option.value)).toEqual(["available", "in_use"]);
    expect(document.activeElement).toBe(status);
    expect(screen.getByText(/Operadores podem alternar o ativo/)).toBeTruthy();
  });

  it("mantém as opções administrativas disponíveis para gestor e administrador", () => {
    renderDialog(false);

    const location = screen.getByLabelText("Unidade") as HTMLSelectElement;
    const status = screen.getByLabelText("Estado") as HTMLSelectElement;

    expect(location.disabled).toBe(false);
    expect(Array.from(status.options, (option) => option.value)).toEqual(["available", "in_use", "maintenance"]);
    expect(document.activeElement).toBe(location);
  });
});
