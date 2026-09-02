import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { CreateReplenishmentRequestInput, ReviewReplenishmentRequestInput } from "../../domain/replenishment";
import { replenishmentGateway } from "../../infrastructure/replenishment-gateway";

export function useReplenishmentRequests() {
  return useQuery({ queryKey: ["replenishment-requests"], queryFn: () => replenishmentGateway.list() });
}
export function useCreateReplenishmentRequest() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: (input: CreateReplenishmentRequestInput) => replenishmentGateway.create(input),
    onSuccess: () => client.invalidateQueries({ queryKey: ["replenishment-requests"] }),
  });
}
export function useReviewReplenishmentRequest() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: (input: ReviewReplenishmentRequestInput) => replenishmentGateway.review(input),
    onSuccess: async () => Promise.all([
      client.invalidateQueries({ queryKey: ["replenishment-requests"] }),
      client.invalidateQueries({ queryKey: ["inventory"] }),
      client.invalidateQueries({ queryKey: ["audit"] }),
    ]),
  });
}
