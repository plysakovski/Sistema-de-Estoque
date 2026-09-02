import AlertTriangle from "lucide-react/dist/esm/icons/alert-triangle.mjs";
import Boxes from "lucide-react/dist/esm/icons/boxes.mjs";
import Building2 from "lucide-react/dist/esm/icons/building-2.mjs";
import Tags from "lucide-react/dist/esm/icons/tags.mjs";
import type { UserRole } from "../../domain/auth";
import { EmptyState } from "../../components/empty-state";
import { Metric } from "../../components/metric";
import { formatDateTime, movementLabel } from "../../shared/format";
import { useDashboard } from "./use-dashboard";

export function DashboardPage({ role, onOpenAdjustments }: { role: UserRole; onOpenAdjustments(): void }) {
  const dashboard = useDashboard();

  if (dashboard.isPending) return <div className="page-state">Carregando visão geral…</div>;
  if (dashboard.isError || !dashboard.data) return <div className="page-state page-state--error">Não foi possível carregar o estoque.</div>;

  const data = dashboard.data;
  const maxQuantity = Math.max(...data.inventoryByLocation.map((entry) => entry.quantity), 1);

  return (
    <div className="dashboard-page">
      <header className="page-heading">
        <div>
          <p className="path-label">C:\ DASHBOARD &gt; VISÃO GERAL</p>
          <h1>Visão geral do estoque</h1>
          <p>Panorama atualizado do seu inventário de TI.</p>
        </div>
        <span className="updated-at">Atualizado em: {formatDateTime(data.updatedAt)}</span>
      </header>

      <section className="metrics" aria-label="Indicadores do estoque">
        <Metric label="Total de itens" value={data.totalItems} icon={Boxes} />
        <Metric label="Unidades" value={data.locationCount} icon={Building2} />
        <Metric label="Categorias" value={data.categoryCount} icon={Tags} />
        <Metric label="Atenção" value={data.attentionCount} icon={AlertTriangle} tone="warning" />
      </section>

      <div className="dashboard-grid">
        <section className="panel stock-chart">
          <h2>&gt; Itens em estoque por unidade</h2>
          <div className="bars">
            {data.inventoryByLocation.map((entry) => (
              <div className="bar-row" key={entry.location}>
                <span>{entry.location}</span>
                <div className="bar-track"><i style={{ width: `${Math.max((entry.quantity / maxQuantity) * 100, 4)}%` }} /></div>
                <strong>{entry.quantity}</strong>
              </div>
            ))}
          </div>
          <div className="chart-legend"><i /> Itens em estoque</div>
        </section>

        <section className="panel attention-panel">
          <h2>&gt; Atenção</h2>
          <div className="attention-title"><span /> Estoque baixo <strong>{data.lowStock.length}</strong></div>
          {data.lowStock.length === 0 ? (
            <EmptyState title="Tudo em ordem" description="Nenhum item abaixo do estoque mínimo." />
          ) : (
            <div className="compact-table" role="table" aria-label="Itens com estoque baixo">
              <div role="row" className="compact-table__head"><span>Item</span><span>Unidade</span><span>Atual</span><span>Mínimo</span></div>
              {data.lowStock.map((item) => (
                <div role="row" key={item.id}><span>{item.name}</span><span>{item.locations}</span><strong>{item.quantity}</strong><span>{item.minimumQuantity}</span></div>
              ))}
            </div>
          )}
          <div className="attention-secondary"><i /> Equipamentos em manutenção <strong>{data.maintenanceCount}</strong></div>
          <p>{data.maintenanceCount === 0 ? "Nenhum equipamento em manutenção." : "Consulte os ativos para acompanhar o atendimento."}</p>
          {role !== "operator" ? <button className="pending-review" onClick={onOpenAdjustments}><span>Solicitações aguardando análise</span><strong>{data.pendingAdjustmentCount}</strong></button> : null}
        </section>
      </div>

      <section className="panel movements-panel">
        <h2>&gt; Movimentações recentes</h2>
        <div className="table-scroll">
          <table>
            <thead><tr><th>Data/Hora</th><th>Tipo</th><th>Item</th><th>Unidade</th><th>Categoria</th><th>Qtd.</th><th>Usuário</th><th>Observação</th></tr></thead>
            <tbody>
              {data.recentMovements.map((movement) => (
                <tr key={movement.id}>
                  <td>{formatDateTime(movement.occurredAt)}</td>
                  <td><span className={`movement movement--${movement.kind}`}>{movementLabel(movement.kind)}</span></td>
                  <td>{movement.itemName}{movement.assetTag ? <small className="table-subline">{movement.assetTag}</small> : null}</td><td>{movement.location}</td><td>{movement.category}</td><td>{movement.quantity}</td><td>{movement.actor}</td><td>{movement.note}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </section>
    </div>
  );
}
