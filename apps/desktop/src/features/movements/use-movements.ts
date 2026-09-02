import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { CreateMovementBatchInput, CreateMovementInput } from "../../domain/inventory";
import { inventoryGateway } from "../../infrastructure/inventory-gateway";

export function useMovements() {
  return useQuery({ queryKey: ["movements"], queryFn: () => inventoryGateway.listMovements() });
}

export function useAssets(productId: string) {
  return useQuery({
    queryKey: ["assets", productId],
    queryFn: () => inventoryGateway.listAssets(productId),
    enabled: productId.length > 0,
  });
}

export function useCreateMovement() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: (input: CreateMovementInput) => inventoryGateway.createMovement(input),
    onSuccess: async (_, input) => {
      await Promise.all([
        client.invalidateQueries({ queryKey: ["movements"] }),
        client.invalidateQueries({ queryKey: ["inventory"] }),
        client.invalidateQueries({ queryKey: ["assets", input.productId] }),
        client.invalidateQueries({ queryKey: ["dashboard"] }),
      ]);
    },
  });
}

export function useCreateMovementBatch() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: (input: CreateMovementBatchInput) => inventoryGateway.createMovementBatch(input),
    onSuccess: async () => {
      await Promise.all([
        client.invalidateQueries({ queryKey: ["movements"] }),
        client.invalidateQueries({ queryKey: ["inventory"] }),
        client.invalidateQueries({ queryKey: ["assets"] }),
        client.invalidateQueries({ queryKey: ["dashboard"] }),
        client.invalidateQueries({ queryKey: ["replenishment-requests"] }),
      ]);
    },
  });
}
