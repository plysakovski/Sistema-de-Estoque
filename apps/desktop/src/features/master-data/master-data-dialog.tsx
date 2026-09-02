import X from "lucide-react/dist/esm/icons/x.mjs";
import { useState, type FormEvent } from "react";
import {
  saveMasterDataInputSchema,
  type MasterDataKind,
  type MasterDataRecord,
} from "../../domain/master-data";
import { useSaveMasterData } from "./use-master-data";

interface MasterDataDialogProps {
  kind: MasterDataKind;
  record: MasterDataRecord | null;
  onClose(): void;
}

const labels = {
  location: { singular: "unidade", title: "Unidade", namePlaceholder: "Ex.: Almoxarifado Norte", codePlaceholder: "Ex.: ALMOX_NORTE" },
  category: { singular: "categoria", title: "Categoria", namePlaceholder: "Ex.: Acessórios", codePlaceholder: "Ex.: ACESSORIOS" },
} as const;

function suggestedCode(name: string): string {
  return name
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toUpperCase()
    .trim()
    .replace(/[^A-Z0-9]+/g, "_")
    .replace(/^_+|_+$/g, "")
    .slice(0, 32);
}

export function MasterDataDialog({ kind, record, onClose }: MasterDataDialogProps) {
  const [name, setName] = useState(record?.name ?? "");
  const [code, setCode] = useState(record?.code ?? "");
  const [codeEdited, setCodeEdited] = useState(Boolean(record));
  const [error, setError] = useState("");
  const save = useSaveMasterData(kind);
  const copy = labels[kind];

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const parsed = saveMasterDataInputSchema.safeParse({ id: record?.id ?? null, name, code });
    if (!parsed.success) {
      setError(parsed.error.issues[0]?.message ?? "Revise os campos.");
      return;
    }
    try {
      setError("");
      await save.mutateAsync(parsed.data);
      onClose();
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    }
  }

  function changeName(value: string) {
    setName(value);
    if (!codeEdited) setCode(suggestedCode(value));
  }

  return (
    <div className="dialog-backdrop" role="presentation" onMouseDown={(event) => { if (event.target === event.currentTarget) onClose(); }}>
      <section className="dialog dialog--compact" role="dialog" aria-modal="true" aria-labelledby="master-data-title">
        <header><div><p className="path-label">C:\ {copy.title.toUpperCase()}S &gt; {record ? "EDITAR" : "NOVA"}</p><h2 id="master-data-title">{record ? "Editar" : "Cadastrar"} {copy.singular}</h2></div><button className="icon-button" type="button" onClick={onClose} aria-label="Fechar"><X /></button></header>
        <form onSubmit={handleSubmit}>
          <div className="form-grid form-grid--single">
            <label><span>Nome</span><input autoFocus value={name} onChange={(event) => changeName(event.target.value)} placeholder={copy.namePlaceholder} /></label>
            <label><span>Código</span><input className="mono" value={code} onChange={(event) => { setCodeEdited(true); setCode(event.target.value.toUpperCase()); }} placeholder={copy.codePlaceholder} /></label>
          </div>
          <p className="form-hint">O código identifica o registro internamente e deve ser único.</p>
          {error ? <p className="form-error" role="alert">{error}</p> : null}
          <footer><button className="button button--secondary" type="button" onClick={onClose}>Cancelar</button><button className="button button--primary" type="submit" disabled={save.isPending}>{save.isPending ? "Salvando…" : "Salvar"}</button></footer>
        </form>
      </section>
    </div>
  );
}
