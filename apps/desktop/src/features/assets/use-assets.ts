import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { AssetFilters, UpdateAssetInput } from "../../domain/assets";
import { assetGateway } from "../../infrastructure/assets-gateway";

export function useAssetRecords(filters: AssetFilters) {
  return useQuery({
    queryKey: ["asset-records", filters],
    queryFn: () => assetGateway.listAssets(filters),
  });
}

export function useUpdateAsset() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: (input: UpdateAssetInput) => assetGateway.updateAsset(input),
    onSuccess: async (asset) => {
      await Promise.all([
        client.invalidateQueries({ queryKey: ["asset-records"] }),
        client.invalidateQueries({ queryKey: ["assets", asset.productId] }),
        client.invalidateQueries({ queryKey: ["inventory"] }),
        client.invalidateQueries({ queryKey: ["movements"] }),
        client.invalidateQueries({ queryKey: ["dashboard"] }),
      ]);
    },
  });
}
