import { z } from "zod";
import { userRoleSchema, type UserRole } from "./auth";

export const userSchema = z.object({
  id: z.string().uuid(), username: z.string(), displayName: z.string(), role: userRoleSchema,
  active: z.boolean(), failedLoginAttempts: z.number().int().nonnegative(), lockedUntil: z.string().nullable(),
  lastLoginAt: z.string().nullable(), createdAt: z.string(), updatedAt: z.string(),
  locationIds: z.array(z.string().uuid()),
});
export type User = z.infer<typeof userSchema>;

export interface CreateUserInput { username: string; displayName: string; role: UserRole; password: string; locationIds: string[] }
export interface UpdateUserInput { id: string; displayName: string; role: UserRole; active: boolean; locationIds: string[] }
export interface ResetUserPasswordInput { id: string; password: string }
export interface UserGateway {
  list(): Promise<User[]>;
  create(input: CreateUserInput): Promise<User>;
  update(input: UpdateUserInput): Promise<User>;
  resetPassword(input: ResetUserPasswordInput): Promise<User>;
}
