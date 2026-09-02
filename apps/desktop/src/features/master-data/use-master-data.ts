import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { MasterDataKind, SaveMasterDataInput } from "../../domain/master-data";
import { masterDataGateway } from "../../infrastructure/master-data-gateway";

export function useMasterData(kind: MasterDataKind) {
  return useQuery({ queryKey: ["master-data", kind], queryFn: () => masterDataGateway.list(kind) });
}

export function useSaveMasterData(kind: MasterDataKind) {
  const client = useQueryClient();
  return useMutation({
    mutationFn: (input: SaveMasterDataInput) => masterDataGateway.save(kind, input),
    onSuccess: async () => {
      await Promise.all([
        client.invalidateQueries({ queryKey: ["master-data", kind] }),
        client.invalidateQueries({ queryKey: ["dashboard"] }),
        kind === "location" ? client.invalidateQueries({ queryKey: ["locations"] }) : Promise.resolve(),
      ]);
    },
  });
}

export function useSetMasterDataActive(kind: MasterDataKind) {
  const client = useQueryClient();
  return useMutation({
    mutationFn: ({ id, active }: { id: string; active: boolean }) => masterDataGateway.setActive(kind, id, active),
    onSuccess: async () => {
      await Promise.all([
        client.invalidateQueries({ queryKey: ["master-data", kind] }),
        client.invalidateQueries({ queryKey: ["dashboard"] }),
        kind === "location" ? client.invalidateQueries({ queryKey: ["locations"] }) : Promise.resolve(),
      ]);
    },
  });
}
