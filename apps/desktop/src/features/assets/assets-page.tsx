import Pencil from "lucide-react/dist/esm/icons/pencil.mjs";
import Search from "lucide-react/dist/esm/icons/search.mjs";
import { useDeferredValue, useMemo, useState } from "react";
import type { UserRole } from "../../domain/auth";
import { countAssetsByStatus, type AssetRecord } from "../../domain/assets";
import type { AssetStatus } from "../../domain/inventory";
import { formatDateTime } from "../../shared/format";
import { useLocations } from "../inventory/use-inventory";
import { EditAssetDialog } from "./edit-asset-dialog";
import { useAssetRecords } from "./use-assets";

const statusLabels: Record<AssetStatus, string> = {
  available: "Disponível",
  in_use: "Em uso",
  maintenance: "Manutenção",
  disposed: "Baixado",
};

export function AssetsPage({ role }: { role: UserRole }) {
  const [search, setSearch] = useState("");
  const [locationId, setLocationId] = useState("");
  const [status, setStatus] = useState<AssetStatus | "">("");
  const [selectedAsset, setSelectedAsset] = useState<AssetRecord | null>(null);
  const deferredSearch = useDeferredValue(search);
  const filters = useMemo(() => ({ search: deferredSearch, locationId: locationId || null, status: status || null }), [deferredSearch, locationId, status]);
  const assets = useAssetRecords(filters);
  const locations = useLocations();
  const summary = countAssetsByStatus(assets.data ?? []);

  return (
    <div>
      <header className="page-heading"><div><p className="path-label">C:\ ATIVOS</p><h1>Ativos patrimoniais</h1><p>Rastreie cada equipamento por patrimônio, série, unidade e estado operacional.</p></div></header>
      <section className="asset-summary" aria-label="Resumo dos ativos filtrados">
        <div><span>Total</span><strong>{summary.total}</strong></div><div><span>Disponíveis</span><strong>{summary.available}</strong></div><div><span>Em uso</span><strong>{summary.in_use}</strong></div><div><span>Manutenção</span><strong>{summary.maintenance}</strong></div><div><span>Baixados</span><strong>{summary.disposed}</strong></div>
      </section>
      <div className="toolbar asset-filters">
        <label className="search-box search-box--page"><Search aria-hidden="true" /><span className="sr-only">Buscar ativos</span><input value={search} onChange={(event) => setSearch(event.target.value)} placeholder="Patrimônio, série, produto, SKU ou categoria" /></label>
        <label><span className="sr-only">Filtrar por unidade</span><select value={locationId} onChange={(event) => setLocationId(event.target.value)}><option value="">Todas as unidades</option>{locations.data?.map((location) => <option key={location.id} value={location.id}>{location.name}</option>)}</select></label>
        <label><span className="sr-only">Filtrar por estado</span><select value={status} onChange={(event) => setStatus(event.target.value as AssetStatus | "")}><option value="">Todos os estados</option>{Object.entries(statusLabels).map(([value, label]) => <option key={value} value={value}>{label}</option>)}</select></label>
      </div>
      <section className="panel inventory-panel">
        <div className="table-scroll"><table><thead><tr><th>Patrimônio</th><th>Produto</th><th>Série</th><th>Categoria</th><th>Unidade</th><th>Estado</th><th>Atualização</th><th><span className="sr-only">Ações</span></th></tr></thead><tbody>{assets.data?.map((asset) => { const editable = asset.status !== "disposed" && (role !== "operator" || asset.status === "available" || asset.status === "in_use"); return <tr key={asset.id} className={asset.status === "disposed" ? "row--inactive" : ""}><td className="mono">{asset.assetTag}</td><td>{asset.productName}<small className="table-subline">{asset.sku}</small></td><td className="mono">{asset.serialNumber ?? "—"}</td><td>{asset.category}</td><td>{asset.location}</td><td><span className={`status status--${asset.status}`}>{statusLabels[asset.status]}</span></td><td>{formatDateTime(asset.updatedAt)}</td><td><div className="table-actions"><button className="icon-button" type="button" disabled={!editable} onClick={() => setSelectedAsset(asset)} aria-label={`Editar ${asset.assetTag}`} title={editable ? "Atualizar ativo" : "Este estado exige perfil gestor"}><Pencil /></button></div></td></tr>; })}</tbody></table></div>
        {assets.isPending ? <div className="page-state">Carregando ativos…</div> : null}
        {assets.isError ? <div className="page-state page-state--error">Não foi possível carregar os ativos.</div> : null}
        {!assets.isPending && assets.data?.length === 0 ? <div className="page-state">Nenhum ativo encontrado para os filtros informados.</div> : null}
      </section>
      <EditAssetDialog key={selectedAsset?.id ?? "closed"} asset={selectedAsset} locations={locations.data ?? []} operatorMode={role === "operator"} onClose={() => setSelectedAsset(null)} />
    </div>
  );
}
