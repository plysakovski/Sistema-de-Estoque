import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { CreateItemInput } from "../../domain/inventory";
import { inventoryGateway } from "../../infrastructure/inventory-gateway";

export function useInventory(search: string) {
  return useQuery({
    queryKey: ["inventory", search],
    queryFn: () => inventoryGateway.listItems(search),
  });
}

export function useLocations() {
  return useQuery({
    queryKey: ["locations"],
    queryFn: () => inventoryGateway.listLocations(),
    staleTime: 60_000,
  });
}

export function useCreateItem() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: (input: CreateItemInput) => inventoryGateway.createItem(input),
    onSuccess: async () => {
      await Promise.all([
        client.invalidateQueries({ queryKey: ["inventory"] }),
        client.invalidateQueries({ queryKey: ["dashboard"] }),
      ]);
    },
  });
}
