import Pencil from "lucide-react/dist/esm/icons/pencil.mjs";
import Plus from "lucide-react/dist/esm/icons/plus.mjs";
import Power from "lucide-react/dist/esm/icons/power.mjs";
import PowerOff from "lucide-react/dist/esm/icons/power-off.mjs";
import { useState } from "react";
import type { MasterDataKind, MasterDataRecord } from "../../domain/master-data";
import { formatDateTime } from "../../shared/format";
import { MasterDataDialog } from "./master-data-dialog";
import { useMasterData, useSetMasterDataActive } from "./use-master-data";

interface MasterDataPageProps {
  kind: MasterDataKind;
}

const pageCopy = {
  location: {
    path: "UNIDADES",
    title: "Unidades",
    description: "Locais físicos que recebem saldos e ativos patrimoniais.",
    button: "Nova unidade",
    usage: "Itens vinculados",
  },
  category: {
    path: "CATEGORIAS",
    title: "Categorias",
    description: "Classificações padronizadas para organizar o catálogo.",
    button: "Nova categoria",
    usage: "Produtos vinculados",
  },
} as const;

export function MasterDataPage({ kind }: MasterDataPageProps) {
  const [editor, setEditor] = useState<MasterDataRecord | null | undefined>(undefined);
  const [actionError, setActionError] = useState("");
  const records = useMasterData(kind);
  const setActive = useSetMasterDataActive(kind);
  const copy = pageCopy[kind];

  async function toggleActive(record: MasterDataRecord) {
    try {
      setActionError("");
      await setActive.mutateAsync({ id: record.id, active: !record.active });
    } catch (reason) {
      setActionError(reason instanceof Error ? reason.message : String(reason));
    }
  }

  return (
    <div>
      <header className="page-heading">
        <div><p className="path-label">C:\ {copy.path}</p><h1>{copy.title}</h1><p>{copy.description}</p></div>
        <button className="button button--primary" onClick={() => setEditor(null)}><Plus aria-hidden="true" /> {copy.button}</button>
      </header>
      {actionError ? <div className="inline-error" role="alert">{actionError}</div> : null}
      <section className="panel inventory-panel">
        <div className="table-scroll">
          <table>
            <thead><tr><th>Nome</th><th>Código</th><th>{copy.usage}</th><th>Status</th><th>Atualização</th><th aria-label="Ações" /></tr></thead>
            <tbody>{records.data?.map((record) => <tr key={record.id} className={record.active ? undefined : "row--inactive"}><td><strong>{record.name}</strong></td><td className="mono">{record.code}</td><td>{record.usageCount}</td><td><span className={`record-status record-status--${record.active ? "active" : "inactive"}`}>{record.active ? "Ativo" : "Inativo"}</span></td><td>{formatDateTime(record.updatedAt)}</td><td><div className="table-actions"><button className="icon-button" type="button" onClick={() => setEditor(record)} aria-label={`Editar ${record.name}`} title="Editar"><Pencil /></button><button className="icon-button" type="button" onClick={() => void toggleActive(record)} disabled={setActive.isPending} aria-label={`${record.active ? "Desativar" : "Ativar"} ${record.name}`} title={record.active ? "Desativar" : "Ativar"}>{record.active ? <PowerOff /> : <Power />}</button></div></td></tr>)}</tbody>
          </table>
        </div>
        {records.isPending ? <div className="page-state">Carregando cadastros…</div> : null}
        {records.isError ? <div className="page-state page-state--error">Não foi possível carregar os cadastros.</div> : null}
        {!records.isPending && records.data?.length === 0 ? <div className="page-state">Nenhum cadastro encontrado.</div> : null}
      </section>
      {editor !== undefined ? <MasterDataDialog kind={kind} record={editor} onClose={() => setEditor(undefined)} /> : null}
    </div>
  );
}
