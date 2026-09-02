import X from "lucide-react/dist/esm/icons/x.mjs";
import { useState, type FormEvent } from "react";
import { updateAssetInputSchema, type AssetRecord, type EditableAssetStatus } from "../../domain/assets";
import type { Location } from "../../domain/inventory";
import { useUpdateAsset } from "./use-assets";

interface EditAssetDialogProps {
  asset: AssetRecord | null;
  locations: Location[];
  operatorMode: boolean;
  onClose(): void;
}

const statusOptions: Array<{ value: EditableAssetStatus; label: string }> = [
  { value: "available", label: "Disponível" },
  { value: "in_use", label: "Em uso" },
  { value: "maintenance", label: "Em manutenção" },
];

export function EditAssetDialog({ asset, locations, operatorMode, onClose }: EditAssetDialogProps) {
  const [locationId, setLocationId] = useState(() => asset?.locationId ?? "");
  const [status, setStatus] = useState<EditableAssetStatus>(() => asset?.status === "disposed" ? "available" : asset?.status ?? "available");
  const [note, setNote] = useState("");
  const [error, setError] = useState("");
  const updateAsset = useUpdateAsset();

  if (!asset) return null;

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!asset) return;
    const parsed = updateAssetInputSchema.safeParse({ id: asset.id, locationId, status, note });
    if (!parsed.success) {
      setError(parsed.error.issues[0]?.message ?? "Revise os dados do ativo.");
      return;
    }
    try {
      setError("");
      await updateAsset.mutateAsync(parsed.data);
      onClose();
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    }
  }

  return (
    <div className="dialog-backdrop" role="presentation" onMouseDown={(event) => { if (event.target === event.currentTarget) onClose(); }}>
      <section className="dialog dialog--compact" role="dialog" aria-modal="true" aria-labelledby="edit-asset-title">
        <header><div><p className="path-label">C:\ ATIVOS &gt; EDITAR</p><h2 id="edit-asset-title">Atualizar {asset.assetTag}</h2></div><button className="icon-button" type="button" onClick={onClose} aria-label="Fechar"><X /></button></header>
        <form onSubmit={handleSubmit}>
          <div className="asset-dialog-summary"><strong>{asset.productName}</strong><span>{asset.sku}{asset.serialNumber ? ` · Série ${asset.serialNumber}` : ""}</span></div>
          <div className="form-grid form-grid--single">
            <label><span>Unidade</span><select autoFocus={!operatorMode} value={locationId} disabled={operatorMode} onChange={(event) => setLocationId(event.target.value)}>{locations.map((location) => <option key={location.id} value={location.id}>{location.name} ({location.code})</option>)}</select></label>
            <label><span>Estado</span><select autoFocus={operatorMode} value={status} onChange={(event) => setStatus(event.target.value as EditableAssetStatus)}>{statusOptions.filter((option) => !operatorMode || option.value !== "maintenance").map((option) => <option key={option.value} value={option.value}>{option.label}</option>)}</select></label>
            <label><span>Observação</span><textarea rows={3} value={note} onChange={(event) => setNote(event.target.value)} placeholder="Motivo da alteração (opcional)" /></label>
          </div>
          <p className="form-hint">{operatorMode ? "Operadores podem alternar o ativo entre Disponível e Em uso dentro da própria unidade." : "A mudança gera movimentação e registro de auditoria automaticamente."}</p>
          {error ? <p className="form-error" role="alert">{error}</p> : null}
          <footer><button type="button" className="button button--secondary" onClick={onClose}>Cancelar</button><button type="submit" className="button button--primary" disabled={updateAsset.isPending}>{updateAsset.isPending ? "Salvando…" : "Salvar alterações"}</button></footer>
        </form>
      </section>
    </div>
  );
}
