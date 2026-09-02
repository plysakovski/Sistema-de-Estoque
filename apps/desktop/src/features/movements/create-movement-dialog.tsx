import Plus from "lucide-react/dist/esm/icons/plus.mjs";
import X from "lucide-react/dist/esm/icons/x.mjs";
import { useState, type FormEvent } from "react";
import type { UserRole } from "../../domain/auth";
import {
  createMovementInputSchema,
  type AssetStatus,
  type InventoryItem,
  type Location,
  type MovementKind,
  type SerialNumberPolicy,
  type TrackingType,
} from "../../domain/inventory";
import type { ReplenishmentRequest } from "../../domain/replenishment";
import { useInventory, useLocations } from "../inventory/use-inventory";
import { buildReceiptLineSeeds } from "../replenishment/receipt-lines";
import { useAssets, useCreateMovementBatch } from "./use-movements";

interface Props {
  open: boolean;
  role: UserRole;
  replenishmentRequest?: ReplenishmentRequest | null;
  onClose(): void;
}

interface Line {
  id: string;
  productId: string;
  productName: string;
  sku: string;
  quantity: string;
  assetId: string;
  assetTag: string;
  serialNumber: string;
  assetStatus: AssetStatus;
  included: boolean;
  trackingType: TrackingType | null;
  serialNumberPolicy: SerialNumberPolicy | null;
  receiptLabel: string | null;
  maxQuantity: number | null;
}

const newLine = (): Line => ({
  id: crypto.randomUUID(), productId: "", productName: "", sku: "", quantity: "1",
  assetId: "", assetTag: "", serialNumber: "", assetStatus: "available", included: true,
  trackingType: null, serialNumberPolicy: null, receiptLabel: null, maxQuantity: null,
});

const labels: Record<MovementKind, string> = {
  entry: "Entrada", exit: "Saída", transfer: "Transferência", adjustment: "Ajuste",
};

export function CreateMovementDialog({ open, role, replenishmentRequest = null, onClose }: Props) {
  const operator = role === "operator";
  const receiptMode = replenishmentRequest !== null;
  const [kind, setKind] = useState<MovementKind>(receiptMode ? "entry" : operator ? "exit" : "entry");
  const [fromLocationId, setFrom] = useState("");
  const [toLocationId, setTo] = useState(replenishmentRequest?.destinationLocationId ?? "");
  const [reference, setReference] = useState("");
  const [note, setNote] = useState("");
  const [lines, setLines] = useState<Line[]>(() => replenishmentRequest
    ? buildReceiptLineSeeds(replenishmentRequest).map((seed) => ({ ...newLine(), ...seed }))
    : [newLine()]);
  const [error, setError] = useState("");
  const inventory = useInventory("");
  const locations = useLocations();
  const create = useCreateMovementBatch();
  if (!open) return null;

  function patchLine(id: string, patch: Partial<Line>) {
    setLines((current) => current.map((line) => line.id === id ? { ...line, ...patch } : line));
  }

  function changeKind(next: MovementKind) {
    setKind(next);
    setLines((current) => current.map((line) => ({ ...line, assetId: "", assetTag: "", serialNumber: "" })));
  }

  async function submit(event: FormEvent) {
    event.preventDefault();
    try {
      const selectedLines = lines.filter((line) => line.included);
      if (selectedLines.length === 0) throw new Error("Selecione ao menos uma unidade recebida nesta entrega");
      const items = selectedLines.map((line) => {
        const product = inventory.data?.find((item) => item.id === line.productId);
        const serialized = (line.trackingType ?? product?.trackingType) === "serialized";
        return createMovementInputSchema.parse({
          productId: line.productId,
          kind,
          quantity: serialized ? 1 : Number(line.quantity),
          fromLocationId: kind === "exit" || kind === "transfer" ? fromLocationId || null : null,
          toLocationId: kind === "entry" || kind === "transfer" || kind === "adjustment" ? toLocationId || null : null,
          assetId: serialized && kind !== "entry" ? line.assetId || null : null,
          assetTag: serialized && kind === "entry" ? line.assetTag || null : null,
          serialNumber: serialized && kind === "entry" ? line.serialNumber || null : null,
          assetStatus: serialized && kind === "adjustment" ? line.assetStatus : null,
          note,
        });
      });
      setError("");
      await create.mutateAsync({
        kind, reference, note,
        replenishmentRequestId: replenishmentRequest?.id ?? null,
        items,
      });
      onClose();
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    }
  }

  const availableKinds = operator ? (["exit"] as MovementKind[]) : (["entry", "exit", "transfer", "adjustment"] as MovementKind[]);
  const selectedCount = lines.filter((line) => line.included).length;
  return <div className="dialog-backdrop"><section className="dialog" role="dialog" aria-modal="true" aria-labelledby="movement-title"><header><div><p className="path-label">{receiptMode ? `REPOSIÇÃO ${replenishmentRequest.id.slice(0, 8)} > RECEBIMENTO` : "C:\\ MOVIMENTAÇÕES > LOTE"}</p><h2 id="movement-title">{receiptMode ? "Registrar recebimento da compra" : "Registrar movimentação em lote"}</h2></div><button className="icon-button" type="button" onClick={onClose} aria-label="Fechar"><X /></button></header><form onSubmit={submit}>
    {receiptMode ? <div className="receipt-summary"><strong>Produtos definidos pela solicitação</strong><span>Destino: {replenishmentRequest.destinationLocation}. Marque somente as unidades que chegaram nesta entrega.</span></div> : <div className="movement-kinds" role="group" aria-label="Tipo de movimentação">{availableKinds.map((value) => <button type="button" key={value} className={kind === value ? `active movement-kind--${value}` : ""} onClick={() => changeKind(value)}>{labels[value]}</button>)}</div>}
    <div className="form-grid">{kind === "exit" || kind === "transfer" ? <LocationSelect label="Unidade de origem" value={fromLocationId} locations={locations.data ?? []} onChange={setFrom} /> : null}{kind === "entry" || kind === "transfer" || kind === "adjustment" ? <LocationSelect label={kind === "transfer" ? "Unidade de destino" : "Unidade"} value={toLocationId} locations={locations.data ?? []} onChange={setTo} disabled={receiptMode} /> : null}<label><span>{receiptMode ? "Nota fiscal / referência do lote" : "Documento / referência"}</span><input required={receiptMode} maxLength={120} value={reference} onChange={(event) => setReference(event.target.value)} placeholder="Ex.: NF 002184" /></label></div>
    <div className="batch-lines">{lines.map((line, index) => <MovementLine key={line.id} line={line} index={index} kind={kind} products={inventory.data ?? []} receiptMode={receiptMode} onChange={(value) => patchLine(line.id, value)} onRemove={!receiptMode && lines.length > 1 ? () => setLines((current) => current.filter((item) => item.id !== line.id)) : undefined} />)}</div>
    {!receiptMode ? <button className="button button--secondary button--compact" type="button" onClick={() => setLines((current) => [...current, newLine()])}><Plus /> Adicionar produto ou ativo</button> : null}
    <label className="batch-note"><span>Observação do lote</span><textarea rows={3} maxLength={1000} value={note} onChange={(event) => setNote(event.target.value)} /></label>
    {error ? <p className="form-error" role="alert">{error}</p> : null}<footer><button type="button" className="button button--secondary" onClick={onClose}>Cancelar</button><button className="button button--primary" disabled={create.isPending || selectedCount === 0}>{create.isPending ? "Registrando…" : receiptMode ? `Registrar ${selectedCount} ${selectedCount === 1 ? "item recebido" : "itens recebidos"}` : `Confirmar ${selectedCount} ${selectedCount === 1 ? "item" : "itens"}`}</button></footer>
  </form></section></div>;
}

function MovementLine({ line, index, kind, products, receiptMode, onChange, onRemove }: { line: Line; index: number; kind: MovementKind; products: InventoryItem[]; receiptMode: boolean; onChange(value: Partial<Line>): void; onRemove?: () => void }) {
  const product = products.find((item) => item.id === line.productId);
  const serialized = (line.trackingType ?? product?.trackingType) === "serialized";
  const serialNumberPolicy = line.serialNumberPolicy ?? product?.serialNumberPolicy;
  const assets = useAssets(serialized && kind !== "entry" ? line.productId : "");
  return <fieldset className={receiptMode && !line.included ? "receipt-line--excluded" : undefined}><legend>{line.receiptLabel ?? `Item ${index + 1}`}</legend>
    {receiptMode ? <label className="receipt-toggle"><input type="checkbox" checked={line.included} onChange={(event) => onChange({ included: event.target.checked })} /><span>Recebido nesta entrega</span></label> : null}
    <div className="form-grid">{receiptMode ? <div className="receipt-product form-field--full"><span>Produto solicitado</span><strong>{line.sku} — {line.productName}</strong></div> : <label className="form-field--full"><span>Produto</span><select required value={line.productId} onChange={(event) => onChange({ productId: event.target.value, assetId: "", quantity: "1" })}><option value="">Selecione</option>{products.map((item) => <option key={item.id} value={item.id}>{item.sku} — {item.name}</option>)}</select></label>}
      {!serialized ? <label><span>{kind === "adjustment" ? "Novo saldo físico" : receiptMode ? "Quantidade recebida" : "Quantidade"}</span><input required={line.included} disabled={!line.included} type="number" min={kind === "adjustment" ? 0 : 1} max={line.maxQuantity ?? undefined} value={line.quantity} onChange={(event) => onChange({ quantity: event.target.value })} /></label> : null}
      {serialized && kind === "entry" ? <><label><span>Patrimônio</span><input required={line.included} disabled={!line.included} value={line.assetTag} onChange={(event) => onChange({ assetTag: event.target.value })} /></label><label><span>Número de série{serialNumberPolicy === "required" ? " *" : " (opcional)"}</span><input required={line.included && serialNumberPolicy === "required"} disabled={!line.included} value={line.serialNumber} onChange={(event) => onChange({ serialNumber: event.target.value })} /></label></> : null}
      {serialized && kind !== "entry" ? <label className="form-field--full"><span>Ativo individual</span><select required value={line.assetId} onChange={(event) => onChange({ assetId: event.target.value })}><option value="">Selecione por patrimônio ou série</option>{assets.data?.map((asset) => <option key={asset.id} value={asset.id}>{asset.assetTag}{asset.serialNumber ? ` · ${asset.serialNumber}` : ""} — {asset.location}</option>)}</select></label> : null}
      {serialized && kind === "adjustment" ? <label><span>Novo estado</span><select value={line.assetStatus} onChange={(event) => onChange({ assetStatus: event.target.value as AssetStatus })}><option value="available">Disponível</option><option value="in_use">Em uso</option><option value="maintenance">Manutenção</option><option value="disposed">Baixado</option></select></label> : null}
    </div>{onRemove ? <button type="button" className="button button--secondary button--compact" onClick={onRemove}>Remover item</button> : null}</fieldset>;
}

function LocationSelect({ label, value, locations, onChange, disabled = false }: { label: string; value: string; locations: Location[]; onChange(value: string): void; disabled?: boolean }) {
  return <label><span>{label}</span><select required disabled={disabled} value={value} onChange={(event) => onChange(event.target.value)}><option value="">Selecione</option>{locations.map((location) => <option key={location.id} value={location.id}>{location.name}</option>)}</select></label>;
}
