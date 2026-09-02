import ArrowLeftRight from "lucide-react/dist/esm/icons/arrow-left-right.mjs";
import { useState } from "react";
import type { UserRole } from "../../domain/auth";
import { formatDateTime, movementLabel } from "../../shared/format";
import { CreateMovementDialog } from "./create-movement-dialog";
import { useMovements } from "./use-movements";

export function MovementsPage({ role }: { role: UserRole }) {
  const [dialogOpen, setDialogOpen] = useState(false);
  const movements = useMovements();

  return (
    <div>
      <header className="page-heading">
        <div><p className="path-label">C:\ MOVIMENTAÇÕES</p><h1>Movimentações de estoque</h1><p>Entradas, saídas, transferências e ajustes com histórico permanente.</p></div>
        <button className="button button--primary" onClick={() => setDialogOpen(true)}><ArrowLeftRight aria-hidden="true" /> {role === "operator" ? "Nova saída" : "Nova movimentação em lote"}</button>
      </header>
      <section className="panel inventory-panel">
        <div className="table-scroll">
          <table>
            <thead><tr><th>Data/Hora</th><th>Tipo</th><th>Produto / ativo</th><th>Unidade</th><th>Categoria</th><th>Qtd.</th><th>Usuário</th><th>Observação</th></tr></thead>
            <tbody>{movements.data?.map((movement) => <tr key={movement.id}><td>{formatDateTime(movement.occurredAt)}</td><td><span className={`movement movement--${movement.kind}`}>{movementLabel(movement.kind)}</span></td><td>{movement.itemName}{movement.assetTag ? <small className="table-subline">{movement.assetTag}</small> : null}</td><td>{movement.location}</td><td>{movement.category}</td><td>{movement.quantity}</td><td>{movement.actor}</td><td>{movement.note || "—"}</td></tr>)}</tbody>
          </table>
        </div>
        {movements.isPending ? <div className="page-state">Carregando movimentações…</div> : null}
        {movements.isError ? <div className="page-state page-state--error">Não foi possível carregar as movimentações.</div> : null}
        {!movements.isPending && movements.data?.length === 0 ? <div className="page-state">Nenhuma movimentação registrada.</div> : null}
      </section>
      <CreateMovementDialog open={dialogOpen} role={role} onClose={() => setDialogOpen(false)} />
    </div>
  );
}
