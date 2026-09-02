import { useQuery } from "@tanstack/react-query";
import type { AuditFilters } from "../../domain/audit";
import { auditGateway } from "../../infrastructure/audit-gateway";

export function useAuditLogs(filters: AuditFilters) {
  return useQuery({
    queryKey: ["audit", filters],
    queryFn: () => auditGateway.listLogs(filters),
  });
}
