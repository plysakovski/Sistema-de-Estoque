import { invoke } from "@tauri-apps/api/core";
import {
  authenticatedUserSchema,
  authStatusSchema,
  type AuthGateway,
  type AuthenticatedUser,
  type BootstrapAdminInput,
  type LoginInput,
} from "../domain/auth";

const mockAdministrator: AuthenticatedUser = {
  id: "123e4567-e89b-42d3-a456-426614174000",
  username: "admin",
  displayName: "Administrador da demonstração",
  role: "admin",
};

function isTauriRuntime(): boolean {
  return "__TAURI_INTERNALS__" in window;
}

class TauriAuthGateway implements AuthGateway {
  async status() {
    return authStatusSchema.parse(await invoke("auth_status"));
  }

  async bootstrap(input: BootstrapAdminInput) {
    return authenticatedUserSchema.parse(await invoke("bootstrap_admin", { input }));
  }

  async login(input: LoginInput) {
    return authenticatedUserSchema.parse(await invoke("login", { input }));
  }

  async logout() {
    await invoke("logout");
  }
}

class MockAuthGateway implements AuthGateway {
  async status() {
    return { setupRequired: false, user: mockAdministrator };
  }

  async bootstrap() {
    return mockAdministrator;
  }

  async login() {
    return mockAdministrator;
  }

  async logout() {}
}

export const authGateway: AuthGateway = isTauriRuntime()
  ? new TauriAuthGateway()
  : new MockAuthGateway();
