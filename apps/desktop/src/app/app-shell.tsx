import Boxes from "lucide-react/dist/esm/icons/boxes.mjs";
import Building2 from "lucide-react/dist/esm/icons/building-2.mjs";
import ChevronLeft from "lucide-react/dist/esm/icons/chevron-left.mjs";
import CircleGauge from "lucide-react/dist/esm/icons/gauge.mjs";
import ClipboardCheck from "lucide-react/dist/esm/icons/clipboard-check.mjs";
import History from "lucide-react/dist/esm/icons/history.mjs";
import Laptop from "lucide-react/dist/esm/icons/laptop.mjs";
import LogOut from "lucide-react/dist/esm/icons/log-out.mjs";
import Menu from "lucide-react/dist/esm/icons/menu.mjs";
import PackagePlus from "lucide-react/dist/esm/icons/package-plus.mjs";
import Search from "lucide-react/dist/esm/icons/search.mjs";
import Settings from "lucide-react/dist/esm/icons/settings.mjs";
import ShieldCheck from "lucide-react/dist/esm/icons/shield-check.mjs";
import Tags from "lucide-react/dist/esm/icons/tags.mjs";
import Users from "lucide-react/dist/esm/icons/users.mjs";
import Wrench from "lucide-react/dist/esm/icons/wrench.mjs";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState, type ComponentType, type SVGProps } from "react";
import type { AuthenticatedUser, UserRole } from "../domain/auth";
import { AuthScreen } from "../features/auth/auth-screen";
import { DashboardPage } from "../features/dashboard/dashboard-page";
import { AdjustmentsPage } from "../features/adjustments/adjustments-page";
import { AssetsPage } from "../features/assets/assets-page";
import { AuditPage } from "../features/audit/audit-page";
import { useDashboard } from "../features/dashboard/use-dashboard";
import { InventoryPage } from "../features/inventory/inventory-page";
import { IncidentsPage } from "../features/incidents/incidents-page";
import { MasterDataPage } from "../features/master-data/master-data-page";
import { MovementsPage } from "../features/movements/movements-page";
import { ReplenishmentPage } from "../features/replenishment/replenishment-page";
import { SettingsPage } from "../features/settings/settings-page";
import { UsersPage } from "../features/users/users-page";
import { authGateway } from "../infrastructure/auth-gateway";

type PageId = "dashboard" | "inventory" | "assets" | "incidents" | "movements" | "adjustments" | "replenishment" | "locations" | "categories" | "audit" | "users" | "settings";
type Icon = ComponentType<SVGProps<SVGSVGElement>>;

const navigation: Array<{ id: PageId; label: string; icon: Icon; minimumRole: UserRole }> = [
  { id: "dashboard", label: "Visão geral", icon: CircleGauge, minimumRole: "operator" },
  { id: "inventory", label: "Estoque", icon: Boxes, minimumRole: "operator" },
  { id: "assets", label: "Ativos", icon: Laptop, minimumRole: "operator" },
  { id: "incidents", label: "Ocorrências", icon: Wrench, minimumRole: "operator" },
  { id: "movements", label: "Movimentações", icon: History, minimumRole: "operator" },
  { id: "adjustments", label: "Ajustes", icon: ClipboardCheck, minimumRole: "operator" },
  { id: "replenishment", label: "Reposição", icon: PackagePlus, minimumRole: "operator" },
  { id: "locations", label: "Unidades", icon: Building2, minimumRole: "admin" },
  { id: "categories", label: "Categorias", icon: Tags, minimumRole: "manager" },
  { id: "audit", label: "Auditoria", icon: ShieldCheck, minimumRole: "manager" },
  { id: "users", label: "Usuários", icon: Users, minimumRole: "admin" },
  { id: "settings", label: "Configurações", icon: Settings, minimumRole: "manager" },
];

export function AppShell() {
  const queryClient = useQueryClient();
  const auth = useQuery({
    queryKey: ["auth-status"],
    queryFn: () => authGateway.status(),
    retry: false,
    staleTime: 0,
  });
  const authenticate = useMutation({
    mutationFn: (input: { username: string; displayName: string; password: string }) =>
      auth.data?.setupRequired
        ? authGateway.bootstrap(input)
        : authGateway.login({ username: input.username, password: input.password }),
    onSuccess: (user) => queryClient.setQueryData(["auth-status"], { setupRequired: false, user }),
  });
  const logout = useMutation({
    mutationFn: () => authGateway.logout(),
    onSuccess: () => {
      queryClient.removeQueries({ predicate: (query) => query.queryKey[0] !== "auth-status" });
      queryClient.setQueryData(["auth-status"], { setupRequired: false, user: null });
    },
  });

  if (auth.isPending) return <main className="auth-page"><p className="page-state">Preparando ambiente seguro…</p></main>;
  if (auth.isError) return <main className="auth-page"><p className="page-state page-state--error">Não foi possível iniciar a camada de segurança: {errorMessage(auth.error)}</p></main>;
  if (auth.data.setupRequired || !auth.data.user) {
    return <AuthScreen setupRequired={auth.data.setupRequired} busy={authenticate.isPending} error={authenticate.isError ? errorMessage(authenticate.error) : null} onSubmit={(input) => authenticate.mutate(input)} />;
  }
  return <AuthenticatedAppShell user={auth.data.user} onLogout={() => logout.mutate()} logoutBusy={logout.isPending} />;
}

function AuthenticatedAppShell({ user, onLogout, logoutBusy }: { user: AuthenticatedUser; onLogout(): void; logoutBusy: boolean }) {
  const [page, setPage] = useState<PageId>("dashboard");
  const [sidebarOpen, setSidebarOpen] = useState(() => window.innerWidth > 720);
  const dashboard = useDashboard();
  const canManage = user.role !== "operator";
  const visibleNavigation = navigation.filter((item) => hasMinimumRole(user.role, item.minimumRole));

  const content = page === "dashboard" ? <DashboardPage role={user.role} onOpenAdjustments={() => navigateTo("adjustments")} /> : page === "inventory" ? <InventoryPage canManage={canManage} /> : page === "assets" ? <AssetsPage role={user.role} /> : page === "incidents" ? <IncidentsPage role={user.role} /> : page === "movements" ? <MovementsPage role={user.role} /> : page === "adjustments" ? <AdjustmentsPage role={user.role} /> : page === "replenishment" ? <ReplenishmentPage role={user.role} /> : page === "locations" ? <MasterDataPage kind="location" /> : page === "categories" ? <MasterDataPage kind="category" /> : page === "audit" ? <AuditPage /> : page === "users" ? <UsersPage /> : <SettingsPage isAdmin={user.role === "admin"} />;

  function navigateTo(nextPage: PageId) {
    setPage(nextPage);
    if (window.innerWidth <= 720) setSidebarOpen(false);
  }

  return (
    <div className={`app-shell ${sidebarOpen ? "" : "app-shell--collapsed"}`}>
      <aside className="sidebar">
        <div className="brand"><span className="brand-mark">›_</span><strong>StockManager Pro</strong></div>
        <nav aria-label="Navegação principal">
          {visibleNavigation.map(({ id, label, icon: IconComponent }) => <button key={id} className={page === id ? "active" : ""} onClick={() => navigateTo(id)} title={sidebarOpen ? undefined : label}><IconComponent aria-hidden="true" /><span>{label}</span></button>)}
        </nav>
        <div className="sidebar-status"><code>C:\STOCKMANAGER_PRO&gt;</code><span><i /> online</span><small>v1.9.0</small></div>
        <button className="collapse-button" onClick={() => setSidebarOpen((current) => !current)} aria-label={sidebarOpen ? "Recolher menu" : "Expandir menu"}>{sidebarOpen ? <ChevronLeft /> : <Menu />}<span>{sidebarOpen ? "Recolher" : "Expandir"}</span></button>
      </aside>
      <div className="main-column">
        <header className="topbar">
          <button className="mobile-menu" onClick={() => setSidebarOpen((current) => !current)} aria-label="Abrir menu"><Menu /></button>
          <label className="search-box"><Search aria-hidden="true" /><span className="sr-only">Busca global</span><input placeholder="Buscar em todos os inventários…" /></label>
          <div className="session-user"><span>{user.displayName}</span><small>{roleLabel(user.role)}</small></div>
          <button className="icon-button" onClick={onLogout} disabled={logoutBusy} aria-label="Sair da conta" title="Sair da conta"><LogOut aria-hidden="true" /></button>
        </header>
        <main>{content}</main>
        <footer className="statusbar"><code>C:\STOCKMANAGER_PRO&gt;</code><span>{dashboard.data?.locationCount ?? "—"} unidades</span><span>{dashboard.data?.categoryCount ?? "—"} categorias</span><span>{dashboard.data?.totalItems ?? "—"} itens</span><strong>Ambiente: PRODUÇÃO <i /></strong></footer>
      </div>
    </div>
  );
}

const roleWeight: Record<UserRole, number> = { operator: 1, manager: 2, admin: 3 };

function hasMinimumRole(role: UserRole, minimumRole: UserRole) {
  return roleWeight[role] >= roleWeight[minimumRole];
}

function roleLabel(role: UserRole) {
  return role === "admin" ? "Administrador" : role === "manager" ? "Gestor" : "Operador";
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}
