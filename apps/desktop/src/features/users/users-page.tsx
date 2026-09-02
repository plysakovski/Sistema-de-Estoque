import Plus from "lucide-react/dist/esm/icons/plus.mjs";
import X from "lucide-react/dist/esm/icons/x.mjs";
import { useState, type FormEvent } from "react";
import type { UserRole } from "../../domain/auth";
import type { User } from "../../domain/users";
import { formatDateTime } from "../../shared/format";
import { useLocations } from "../inventory/use-inventory";
import { useCreateUser, useResetUserPassword, useUpdateUser, useUsers } from "./use-users";

export function UsersPage() {
  const [creating, setCreating] = useState(false);
  const [editing, setEditing] = useState<User | null>(null);
  const users = useUsers();
  return <div><header className="page-heading"><div><p className="path-label">C:\ ADMINISTRAÇÃO &gt; USUÁRIOS</p><h1>Usuários e acessos</h1><p>Contas individuais, perfis hierárquicos e bloqueios de autenticação.</p></div><button className="button button--primary" onClick={() => setCreating(true)}><Plus /> Novo usuário</button></header><section className="panel inventory-panel"><div className="table-scroll"><table><thead><tr><th>Usuário</th><th>Nome</th><th>Perfil</th><th>Situação</th><th>Falhas</th><th>Último acesso</th><th>Ação</th></tr></thead><tbody>{users.data?.map((user) => <tr key={user.id}><td className="mono">{user.username}</td><td>{user.displayName}</td><td>{roleLabel(user.role)}</td><td><span className={`request-status request-status--${user.active ? "approved" : "rejected"}`}>{user.active ? user.lockedUntil ? "Bloqueado temporariamente" : "Ativo" : "Inativo"}</span></td><td>{user.failedLoginAttempts}</td><td>{user.lastLoginAt ? formatDateTime(user.lastLoginAt) : "Nunca"}</td><td><button className="button button--compact button--secondary" onClick={() => setEditing(user)}>Gerenciar</button></td></tr>)}</tbody></table></div>{users.isPending ? <div className="page-state">Carregando usuários…</div> : null}{users.isError ? <div className="page-state page-state--error">Não foi possível carregar os usuários.</div> : null}</section><CreateUserDialog open={creating} onClose={() => setCreating(false)} /><EditUserDialog user={editing} onClose={() => setEditing(null)} /></div>;
}

function CreateUserDialog({ open, onClose }: { open: boolean; onClose(): void }) {
  const [username, setUsername] = useState(""); const [displayName, setDisplayName] = useState(""); const [role, setRole] = useState<UserRole>("operator"); const [password, setPassword] = useState(""); const [locationIds, setLocationIds] = useState<string[]>([]); const [error, setError] = useState(""); const create = useCreateUser(); const locations = useLocations();
  if (!open) return null;
  async function submit(event: FormEvent) { event.preventDefault(); try { setError(""); await create.mutateAsync({ username, displayName, role, password, locationIds: role === "admin" ? [] : locationIds }); setUsername(""); setDisplayName(""); setPassword(""); setLocationIds([]); onClose(); } catch (reason) { setError(errorText(reason)); } }
  return <div className="dialog-backdrop"><section className="dialog dialog--compact" role="dialog" aria-modal="true"><header><div><p className="path-label">CONTA INDIVIDUAL</p><h2>Novo usuário</h2></div><button className="icon-button" onClick={onClose}><X /></button></header><form onSubmit={submit}><div className="form-grid form-grid--single"><label>Nome completo<input required minLength={3} maxLength={100} value={displayName} onChange={(event) => setDisplayName(event.target.value)} /></label><label>Usuário<input required minLength={3} maxLength={64} value={username} onChange={(event) => setUsername(event.target.value)} /></label><label>Perfil<select value={role} onChange={(event) => setRole(event.target.value as UserRole)}><option value="operator">Operador</option><option value="manager">Gestor</option><option value="admin">Administrador</option></select></label>{role !== "admin" ? <LocationScope locations={locations.data ?? []} selected={locationIds} onChange={setLocationIds} /> : null}<label>Senha inicial<input type="password" autoComplete="new-password" required minLength={12} maxLength={128} value={password} onChange={(event) => setPassword(event.target.value)} /></label></div>{error ? <p className="form-error">{error}</p> : null}<footer><button type="button" className="button button--secondary" onClick={onClose}>Cancelar</button><button className="button button--primary" disabled={create.isPending}>Criar usuário</button></footer></form></section></div>;
}

function EditUserDialog({ user, onClose }: { user: User | null; onClose(): void }) {
  if (!user) return null;
  return <EditUserDialogContent key={user.id} user={user} onClose={onClose} />;
}

function EditUserDialogContent({ user, onClose }: { user: User; onClose(): void }) {
  const [displayName, setDisplayName] = useState(user.displayName); const [role, setRole] = useState<UserRole>(user.role); const [active, setActive] = useState(user.active); const [password, setPassword] = useState(""); const [locationIds, setLocationIds] = useState(user.locationIds); const [error, setError] = useState(""); const update = useUpdateUser(); const reset = useResetUserPassword(); const locations = useLocations();
  async function submit(event: FormEvent) { event.preventDefault(); try { setError(""); await update.mutateAsync({ id: user.id, displayName, role, active, locationIds: role === "admin" ? [] : locationIds }); if (password) await reset.mutateAsync({ id: user.id, password }); onClose(); } catch (reason) { setError(errorText(reason)); } }
  return <div className="dialog-backdrop"><section className="dialog dialog--compact" role="dialog" aria-modal="true"><header><div><p className="path-label">USUÁRIO {user.username}</p><h2>Gerenciar acesso</h2></div><button className="icon-button" onClick={onClose}><X /></button></header><form onSubmit={submit}><div className="form-grid form-grid--single"><label>Nome completo<input required minLength={3} maxLength={100} value={displayName} onChange={(event) => setDisplayName(event.target.value)} /></label><label>Perfil<select value={role} onChange={(event) => setRole(event.target.value as UserRole)}><option value="operator">Operador</option><option value="manager">Gestor</option><option value="admin">Administrador</option></select></label>{role !== "admin" ? <LocationScope locations={locations.data ?? []} selected={locationIds} onChange={setLocationIds} /> : null}<label>Situação<select value={active ? "active" : "inactive"} onChange={(event) => setActive(event.target.value === "active")}><option value="active">Ativo</option><option value="inactive">Inativo</option></select></label><label>Nova senha (opcional)<input type="password" autoComplete="new-password" minLength={12} maxLength={128} value={password} onChange={(event) => setPassword(event.target.value)} /></label></div>{error ? <p className="form-error">{error}</p> : null}<footer><button type="button" className="button button--secondary" onClick={onClose}>Cancelar</button><button className="button button--primary" disabled={update.isPending || reset.isPending}>Salvar alterações</button></footer></form></section></div>;
}

function LocationScope({ locations, selected, onChange }: { locations: Array<{ id: string; name: string }>; selected: string[]; onChange(value: string[]): void }) {
  return <fieldset><legend>Unidades permitidas</legend>{locations.map((location) => <label key={location.id}><input type="checkbox" checked={selected.includes(location.id)} onChange={(event) => onChange(event.target.checked ? [...selected, location.id] : selected.filter((id) => id !== location.id))} /> {location.name}</label>)}</fieldset>;
}

function roleLabel(role: UserRole) { return role === "admin" ? "Administrador" : role === "manager" ? "Gestor" : "Operador"; }
function errorText(reason: unknown) { return reason instanceof Error ? reason.message : String(reason); }
