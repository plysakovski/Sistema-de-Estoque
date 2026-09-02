import type { SerialNumberPolicy, TrackingType } from "../../domain/inventory";
import type { ReplenishmentRequest } from "../../domain/replenishment";

export interface ReceiptLineSeed {
  productId: string;
  productName: string;
  sku: string;
  quantity: string;
  trackingType: TrackingType;
  serialNumberPolicy: SerialNumberPolicy;
  receiptLabel: string;
  maxQuantity: number;
}

export function buildReceiptLineSeeds(request: ReplenishmentRequest): ReceiptLineSeed[] {
  return request.items.flatMap((item) => {
    const remaining = item.purchaseQuantity - item.receivedQuantity;
    if (remaining <= 0) return [];
    const base = {
      productId: item.productId,
      productName: item.productName,
      sku: item.sku,
      trackingType: item.trackingType,
      serialNumberPolicy: item.serialNumberPolicy,
    };
    if (item.trackingType === "serialized") {
      return Array.from({ length: remaining }, (_, index) => ({
        ...base,
        quantity: "1",
        receiptLabel: `${item.productName} · unidade ${index + 1} de ${remaining}`,
        maxQuantity: 1,
      }));
    }
    return [{
      ...base,
      quantity: String(remaining),
      receiptLabel: `${item.productName} · ${remaining} ${remaining === 1 ? "unidade pendente" : "unidades pendentes"}`,
      maxQuantity: remaining,
    }];
  });
}
