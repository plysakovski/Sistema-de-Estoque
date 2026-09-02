import Search from "lucide-react/dist/esm/icons/search.mjs";
import Plus from "lucide-react/dist/esm/icons/plus.mjs";
import { useDeferredValue, useState } from "react";
import { formatDateTime } from "../../shared/format";
import { useInventory } from "./use-inventory";
import { CreateItemDialog } from "./create-item-dialog";

export function InventoryPage({ canManage }: { canManage: boolean }) {
  const [search, setSearch] = useState("");
  const deferredSearch = useDeferredValue(search);
  const inventory = useInventory(deferredSearch);
  const [creating, setCreating] = useState(false);

  return (
    <div>
      <header className="page-heading">
        <div><p className="path-label">C:\ ESTOQUE</p><h1>Catálogo e estoque</h1><p>Cadastre os tipos de produto aqui; quantidades e ativos entram por movimentação.</p></div>
        {canManage ? <button className="button button--primary" onClick={() => setCreating(true)}><Plus /> Cadastrar produto</button> : null}
      </header>
      <div className="toolbar">
        <label className="search-box search-box--page"><Search aria-hidden="true" /><span className="sr-only">Buscar no estoque</span><input value={search} onChange={(event) => setSearch(event.target.value)} placeholder="Produto, SKU, patrimônio, série ou categoria" /></label>
        <span>{inventory.data?.length ?? 0} produtos</span>
      </div>
      <section className="panel inventory-panel">
        <div className="table-scroll">
          <table>
            <thead><tr><th>SKU</th><th>Produto</th><th>Controle</th><th>Categoria</th><th>Distribuição</th><th>Total</th><th>Mínimo</th><th>Atualização</th></tr></thead>
            <tbody>
              {inventory.data?.map((item) => (
                <tr key={item.id}><td className="mono">{item.sku}</td><td>{item.name}</td><td><span className={`tracking tracking--${item.trackingType}`}>{item.trackingType === "quantity" ? "Quantidade" : "Patrimonial"}</span></td><td>{item.category}</td><td>{item.locations}</td><td>{item.quantity}</td><td>{item.trackingType === "quantity" ? item.minimumQuantity : "—"}</td><td>{formatDateTime(item.updatedAt)}</td></tr>
              ))}
            </tbody>
          </table>
        </div>
        {inventory.isPending ? <div className="page-state">Carregando itens…</div> : null}
        {!inventory.isPending && inventory.data?.length === 0 ? <div className="page-state">Nenhum item encontrado.</div> : null}
      </section>
      <CreateItemDialog open={creating} onClose={() => setCreating(false)} />
    </div>
  );
}
