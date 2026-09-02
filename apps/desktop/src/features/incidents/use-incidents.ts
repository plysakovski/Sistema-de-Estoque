import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { CreateAssetIncidentInput, ReviewAssetIncidentInput } from "../../domain/asset-incidents";
import { assetIncidentGateway } from "../../infrastructure/asset-incident-gateway";
export function useIncidents() { return useQuery({ queryKey: ["asset-incidents"], queryFn: () => assetIncidentGateway.list() }); }
export function useCreateIncident() { const client = useQueryClient(); return useMutation({ mutationFn: (input: CreateAssetIncidentInput) => assetIncidentGateway.create(input), onSuccess: async () => Promise.all([client.invalidateQueries({ queryKey: ["asset-incidents"] }), client.invalidateQueries({ queryKey: ["assets"] }), client.invalidateQueries({ queryKey: ["dashboard"] })]) }); }
export function useReviewIncident() { const client = useQueryClient(); return useMutation({ mutationFn: (input: ReviewAssetIncidentInput) => assetIncidentGateway.review(input), onSuccess: async () => Promise.all([client.invalidateQueries({ queryKey: ["asset-incidents"] }), client.invalidateQueries({ queryKey: ["assets"] }), client.invalidateQueries({ queryKey: ["dashboard"] })]) }); }
