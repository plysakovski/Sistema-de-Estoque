import { useQuery } from "@tanstack/react-query";
import { inventoryGateway } from "../../infrastructure/inventory-gateway";

export function useDashboard() {
  return useQuery({
    queryKey: ["dashboard"],
    queryFn: () => inventoryGateway.getDashboard(),
  });
}
