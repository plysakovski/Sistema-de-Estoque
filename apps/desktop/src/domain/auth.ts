import { z } from "zod";

export const userRoleSchema = z.enum(["operator", "manager", "admin"]);
export type UserRole = z.infer<typeof userRoleSchema>;

export const authenticatedUserSchema = z.object({
  id: z.string().uuid(),
  username: z.string(),
  displayName: z.string(),
  role: userRoleSchema,
});
export type AuthenticatedUser = z.infer<typeof authenticatedUserSchema>;

export const authStatusSchema = z.object({
  setupRequired: z.boolean(),
  user: authenticatedUserSchema.nullable(),
});
export type AuthStatus = z.infer<typeof authStatusSchema>;

export interface LoginInput {
  username: string;
  password: string;
}

export interface BootstrapAdminInput extends LoginInput {
  displayName: string;
}

export interface AuthGateway {
  status(): Promise<AuthStatus>;
  bootstrap(input: BootstrapAdminInput): Promise<AuthenticatedUser>;
  login(input: LoginInput): Promise<AuthenticatedUser>;
  logout(): Promise<void>;
}
