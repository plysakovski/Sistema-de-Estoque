import Eye from "lucide-react/dist/esm/icons/eye.mjs";
import Search from "lucide-react/dist/esm/icons/search.mjs";
import { useDeferredValue, useMemo, useState } from "react";
import type { AuditLog } from "../../domain/audit";
import { formatDateTime } from "../../shared/format";
import { AuditDetailsDialog } from "./audit-details-dialog";
import { useAuditLogs } from "./use-audit";

const actionLabels: Record<string, string> = { create: "Criação", update: "Alteração", activate: "Ativação", deactivate: "Desativação", entry: "Entrada", exit: "Saída", transfer: "Transferência", adjustment: "Ajuste" };
const entityLabels: Record<string, string> = { product: "Produto", asset: "Ativo", stock: "Estoque", location: "Unidade", category: "Categoria" };

export function AuditPage() {
  const [search, setSearch] = useState("");
  const [entityType, setEntityType] = useState("");
  const [action, setAction] = useState("");
  const [dateFrom, setDateFrom] = useState("");
  const [dateTo, setDateTo] = useState("");
  const [selectedLog, setSelectedLog] = useState<AuditLog | null>(null);
  const deferredSearch = useDeferredValue(search);
  const filters = useMemo(() => ({ search: deferredSearch, entityType: entityType || null, action: action || null, dateFrom: dateFrom || null, dateTo: dateTo || null }), [action, dateFrom, dateTo, deferredSearch, entityType]);
  const logs = useAuditLogs(filters);
  const uniqueActors = new Set(logs.data?.map((log) => log.actor)).size;

  return (
    <div>
      <header className="page-heading"><div><p className="path-label">C:\ AUDITORIA</p><h1>Auditoria do sistema</h1><p>Histórico imutável das operações realizadas no estoque e nos cadastros.</p></div><span className="audit-readonly">Somente leitura</span></header>
      <section className="audit-summary"><div><span>Eventos filtrados</span><strong>{logs.data?.length ?? 0}</strong></div><div><span>Responsáveis</span><strong>{uniqueActors}</strong></div><div><span>Retenção exibida</span><strong>500</strong><small>eventos mais recentes</small></div></section>
      <div className="audit-filters">
        <label className="search-box search-box--page"><Search aria-hidden="true" /><span className="sr-only">Buscar auditoria</span><input value={search} onChange={(event) => setSearch(event.target.value)} placeholder="Responsável, ação, entidade ou identificador" /></label>
        <label><span>Entidade</span><select value={entityType} onChange={(event) => setEntityType(event.target.value)}><option value="">Todas</option>{Object.entries(entityLabels).map(([value, label]) => <option key={value} value={value}>{label}</option>)}</select></label>
        <label><span>Ação</span><select value={action} onChange={(event) => setAction(event.target.value)}><option value="">Todas</option>{Object.entries(actionLabels).map(([value, label]) => <option key={value} value={value}>{label}</option>)}</select></label>
        <label><span>De</span><input type="date" value={dateFrom} max={dateTo || undefined} onChange={(event) => setDateFrom(event.target.value)} /></label>
        <label><span>Até</span><input type="date" value={dateTo} min={dateFrom || undefined} onChange={(event) => setDateTo(event.target.value)} /></label>
      </div>
      <section className="panel inventory-panel"><div className="table-scroll"><table><thead><tr><th>Data/Hora</th><th>Ação</th><th>Entidade</th><th>Registro</th><th>Responsável</th><th><span className="sr-only">Detalhes</span></th></tr></thead><tbody>{logs.data?.map((log) => <tr key={log.id}><td>{formatDateTime(log.occurredAt)}</td><td><span className={`audit-action audit-action--${log.action}`}>{actionLabels[log.action] ?? log.action}</span></td><td>{entityLabels[log.entityType] ?? log.entityType}</td><td>{log.entityName}<small className="table-subline">{log.entityId}</small></td><td>{log.actor}</td><td><div className="table-actions"><button className="icon-button" type="button" onClick={() => setSelectedLog(log)} aria-label={`Ver detalhes de ${log.entityName}`} title="Ver detalhes"><Eye /></button></div></td></tr>)}</tbody></table></div>{logs.isPending ? <div className="page-state">Carregando auditoria…</div> : null}{logs.isError ? <div className="page-state page-state--error">Não foi possível carregar a auditoria.</div> : null}{!logs.isPending && logs.data?.length === 0 ? <div className="page-state">Nenhum evento encontrado para os filtros informados.</div> : null}</section>
      <AuditDetailsDialog log={selectedLog} onClose={() => setSelectedLog(null)} />
    </div>
  );
}
