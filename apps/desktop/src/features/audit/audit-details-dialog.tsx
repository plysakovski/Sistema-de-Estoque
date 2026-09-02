import X from "lucide-react/dist/esm/icons/x.mjs";
import { buildAuditChanges, type AuditLog } from "../../domain/audit";
import { formatDateTime } from "../../shared/format";

interface AuditDetailsDialogProps {
  log: AuditLog | null;
  onClose(): void;
}

const actionLabels: Record<string, string> = { create: "Criação", update: "Alteração", activate: "Ativação", deactivate: "Desativação", entry: "Entrada", exit: "Saída", transfer: "Transferência", adjustment: "Ajuste" };

export function AuditDetailsDialog({ log, onClose }: AuditDetailsDialogProps) {
  if (!log) return null;
  const changes = buildAuditChanges(log);

  return (
    <div className="dialog-backdrop" role="presentation" onMouseDown={(event) => { if (event.target === event.currentTarget) onClose(); }}>
      <section className="dialog audit-dialog" role="dialog" aria-modal="true" aria-labelledby="audit-detail-title">
        <header><div><p className="path-label">C:\ AUDITORIA &gt; EVENTO</p><h2 id="audit-detail-title">Detalhes do evento</h2></div><button className="icon-button" type="button" onClick={onClose} aria-label="Fechar"><X /></button></header>
        <div className="audit-detail-content">
          <dl className="audit-metadata"><div><dt>Entidade</dt><dd>{log.entityName}</dd></div><div><dt>Ação</dt><dd>{actionLabels[log.action] ?? log.action}</dd></div><div><dt>Responsável</dt><dd>{log.actor}</dd></div><div><dt>Data e hora</dt><dd>{formatDateTime(log.occurredAt)}</dd></div><div className="audit-metadata--wide"><dt>Identificador imutável</dt><dd className="mono">{log.entityId}</dd></div></dl>
          <div className="table-scroll"><table><thead><tr><th>Campo</th><th>Antes</th><th>Depois</th></tr></thead><tbody>{changes.map((change) => <tr key={change.field}><td>{change.field}</td><td className="audit-value">{change.before}</td><td className="audit-value audit-value--after">{change.after}</td></tr>)}</tbody></table></div>
          {changes.length === 0 ? <div className="page-state">Este evento não possui dados comparáveis.</div> : null}
        </div>
      </section>
    </div>
  );
}
