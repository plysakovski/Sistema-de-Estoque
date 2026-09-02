import { invoke } from "@tauri-apps/api/core";
import { userSchema, type CreateUserInput, type ResetUserPasswordInput, type UpdateUserInput, type User, type UserGateway } from "../domain/users";

function isTauriRuntime() { return "__TAURI_INTERNALS__" in window; }

class TauriUserGateway implements UserGateway {
  async list() { return userSchema.array().parse(await invoke("list_users")); }
  async create(input: CreateUserInput) { return userSchema.parse(await invoke("create_user", { input })); }
  async update(input: UpdateUserInput) { return userSchema.parse(await invoke("update_user", { input })); }
  async resetPassword(input: ResetUserPasswordInput) { return userSchema.parse(await invoke("reset_user_password", { input })); }
}

class MockUserGateway implements UserGateway {
  private readonly users: User[] = [{ id: "123e4567-e89b-42d3-a456-426614174000", username: "admin", displayName: "Administrador da demonstração", role: "admin", active: true, failedLoginAttempts: 0, lockedUntil: null, lastLoginAt: new Date().toISOString(), createdAt: new Date().toISOString(), updatedAt: new Date().toISOString(), locationIds: [] }];
  async list() { return structuredClone(this.users); }
  async create(input: CreateUserInput) { const user: User = { id: crypto.randomUUID(), username: input.username, displayName: input.displayName, role: input.role, active: true, failedLoginAttempts: 0, lockedUntil: null, lastLoginAt: null, createdAt: new Date().toISOString(), updatedAt: new Date().toISOString(), locationIds: input.locationIds }; this.users.push(user); return structuredClone(user); }
  async update(input: UpdateUserInput) { const user = this.users.find((entry) => entry.id === input.id); if (!user) throw new Error("Usuário não encontrado"); Object.assign(user, input, { updatedAt: new Date().toISOString() }); return structuredClone(user); }
  async resetPassword(input: ResetUserPasswordInput) { const user = this.users.find((entry) => entry.id === input.id); if (!user) throw new Error("Usuário não encontrado"); return structuredClone(user); }
}

export const userGateway: UserGateway = isTauriRuntime() ? new TauriUserGateway() : new MockUserGateway();
