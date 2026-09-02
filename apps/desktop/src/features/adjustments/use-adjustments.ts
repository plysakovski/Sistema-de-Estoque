import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { CreateAdjustmentRequestInput, ReviewAdjustmentRequestInput } from "../../domain/adjustments";
import { adjustmentGateway } from "../../infrastructure/adjustment-gateway";

export function useAdjustments() {
  return useQuery({ queryKey: ["adjustment-requests"], queryFn: () => adjustmentGateway.list() });
}

export function useCreateAdjustment() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: (input: CreateAdjustmentRequestInput) => adjustmentGateway.create(input),
    onSuccess: async () => {
      await Promise.all([
        client.invalidateQueries({ queryKey: ["adjustment-requests"] }),
        client.invalidateQueries({ queryKey: ["dashboard"] }),
      ]);
    },
  });
}

export function useReviewAdjustment() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: (input: ReviewAdjustmentRequestInput) => adjustmentGateway.review(input),
    onSuccess: async () => {
      await Promise.all([
        client.invalidateQueries({ queryKey: ["adjustment-requests"] }),
        client.invalidateQueries({ queryKey: ["dashboard"] }),
        client.invalidateQueries({ queryKey: ["inventory"] }),
        client.invalidateQueries({ queryKey: ["assets"] }),
        client.invalidateQueries({ queryKey: ["movements"] }),
        client.invalidateQueries({ queryKey: ["audit"] }),
      ]);
    },
  });
}
