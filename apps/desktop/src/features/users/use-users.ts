import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { CreateUserInput, ResetUserPasswordInput, UpdateUserInput } from "../../domain/users";
import { userGateway } from "../../infrastructure/user-gateway";

export function useUsers() { return useQuery({ queryKey: ["users"], queryFn: () => userGateway.list() }); }
function useRefreshUsers() { const client = useQueryClient(); return () => client.invalidateQueries({ queryKey: ["users"] }); }
export function useCreateUser() { const refresh = useRefreshUsers(); return useMutation({ mutationFn: (input: CreateUserInput) => userGateway.create(input), onSuccess: refresh }); }
export function useUpdateUser() { const refresh = useRefreshUsers(); return useMutation({ mutationFn: (input: UpdateUserInput) => userGateway.update(input), onSuccess: refresh }); }
export function useResetUserPassword() { const refresh = useRefreshUsers(); return useMutation({ mutationFn: (input: ResetUserPasswordInput) => userGateway.resetPassword(input), onSuccess: refresh }); }
