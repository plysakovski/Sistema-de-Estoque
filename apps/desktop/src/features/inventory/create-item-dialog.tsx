import X from "lucide-react/dist/esm/icons/x.mjs";
import { useState, type FormEvent } from "react";
import { createItemInputSchema, type SerialNumberPolicy, type TrackingType } from "../../domain/inventory";
import { useCreateItem } from "./use-inventory";

interface CreateItemDialogProps {
  open: boolean;
  onClose(): void;
}

const initialForm = {
  sku: "",
  name: "",
  category: "",
  trackingType: "quantity" as TrackingType,
  serialNumberPolicy: "not_applicable" as SerialNumberPolicy,
  minimumQuantity: "0",
};

export function CreateItemDialog({ open, onClose }: CreateItemDialogProps) {
  const [form, setForm] = useState(initialForm);
  const [error, setError] = useState("");
  const createItem = useCreateItem();

  if (!open) return null;

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const parsed = createItemInputSchema.safeParse({
      ...form,
      locationId: null,
      initialQuantity: 0,
      minimumQuantity: Number(form.minimumQuantity),
      assetTag: null,
      serialNumber: null,
    });
    if (!parsed.success) {
      setError(parsed.error.issues[0]?.message ?? "Revise os campos do cadastro.");
      return;
    }
    try {
      setError("");
      await createItem.mutateAsync(parsed.data);
      setForm(initialForm);
      onClose();
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    }
  }

  function changeTrackingType(trackingType: TrackingType) {
    setForm((current) => ({
      ...current,
      trackingType,
      serialNumberPolicy: trackingType === "serialized" ? "required" : "not_applicable",
      minimumQuantity: trackingType === "serialized" ? "0" : current.minimumQuantity,
    }));
  }

  return (
    <div className="dialog-backdrop" role="presentation" onMouseDown={(event) => { if (event.target === event.currentTarget) onClose(); }}>
      <section className="dialog" role="dialog" aria-modal="true" aria-labelledby="create-item-title">
        <header>
          <div><p className="path-label">C:\ ESTOQUE &gt; NOVO</p><h2 id="create-item-title">Cadastrar produto</h2></div>
          <button className="icon-button" type="button" onClick={onClose} aria-label="Fechar"><X /></button>
        </header>
        <form onSubmit={handleSubmit}>
          <p className="form-hint">Este cadastro cria somente o produto no catálogo. O saldo será incluído posteriormente por uma entrada ou recebimento.</p>
          <div className="tracking-picker" role="group" aria-label="Modelo de controle">
            <button type="button" className={form.trackingType === "quantity" ? "active" : ""} onClick={() => changeTrackingType("quantity")}><strong>Por quantidade</strong><span>Materiais e produtos intercambiáveis</span></button>
            <button type="button" className={form.trackingType === "serialized" ? "active" : ""} onClick={() => changeTrackingType("serialized")}><strong>Ativo individual</strong><span>Equipamentos com patrimônio único</span></button>
          </div>
          <div className="form-grid">
            <label><span>Código / SKU</span><input autoFocus value={form.sku} onChange={(event) => setForm((current) => ({ ...current, sku: event.target.value }))} placeholder="Ex.: NOTE-E14" /></label>
            <label><span>Categoria</span><input value={form.category} onChange={(event) => setForm((current) => ({ ...current, category: event.target.value }))} placeholder="Ex.: Notebooks" /></label>
            <label className="form-field--full"><span>Nome do produto</span><input value={form.name} onChange={(event) => setForm((current) => ({ ...current, name: event.target.value }))} placeholder="Ex.: Notebook Lenovo ThinkPad E14" /></label>
            {form.trackingType === "quantity" ? (
              <label><span>Estoque mínimo</span><input type="number" min="0" value={form.minimumQuantity} onChange={(event) => setForm((current) => ({ ...current, minimumQuantity: event.target.value }))} /></label>
            ) : (
              <label><span>Política do número de série</span><select value={form.serialNumberPolicy} onChange={(event) => setForm((current) => ({ ...current, serialNumberPolicy: event.target.value as SerialNumberPolicy }))}><option value="required">Obrigatório em cada ativo</option><option value="optional">Opcional</option></select></label>
            )}
          </div>
          {error ? <p className="form-error" role="alert">{error}</p> : null}
          <footer><button className="button button--secondary" type="button" onClick={onClose}>Cancelar</button><button className="button button--primary" type="submit" disabled={createItem.isPending}>{createItem.isPending ? "Salvando…" : "Salvar no catálogo"}</button></footer>
        </form>
      </section>
    </div>
  );
}
