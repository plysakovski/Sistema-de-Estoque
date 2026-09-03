import Plus from "lucide-react/dist/esm/icons/plus.mjs";
import X from "lucide-react/dist/esm/icons/x.mjs";
import { useState, type FormEvent } from "react";
import type { UserRole } from "../../domain/auth";
import type { ReplenishmentPriority, ReplenishmentRequest, ReplenishmentTransferShipment } from "../../domain/replenishment";
import { formatDateTime } from "../../shared/format";
import { useInventory, useLocations } from "../inventory/use-inventory";
import {
  useCreateReplenishmentRequest,
  useDispatchReplenishmentTransfer,
  useReceiveReplenishmentTransfer,
  useReplenishmentRequests,
  useReviewReplenishmentRequest,
} from "./use-replenishment";
import { CreateMovementDialog } from "../movements/create-movement-dialog";

export function ReplenishmentPage({ role }: { role: UserRole }) {
  const [creating, setCreating] = useState(false);
  const [reviewing, setReviewing] = useState<ReplenishmentRequest | null>(null);
  const [receiving, setReceiving] = useState<ReplenishmentRequest | null>(null);
  const [dispatching, setDispatching] = useState<ReplenishmentRequest | null>(null);
  const [receivingTransfer, setReceivingTransfer] = useState<ReplenishmentTransferShipment | null>(null);
  const requests = useReplenishmentRequests();
  const canReview = role !== "operator";
  return <div><header className="page-heading"><div><p className="path-label">C:\ SOLICITAÇÕES &gt; REPOSIÇÃO</p><h1>Reposição de produtos</h1><p>{canReview ? "Decida entre transferência interna, compra externa ou atendimento parcial." : "Solicite produtos e confirme transferências destinadas à sua unidade."}</p></div><button className="button button--primary" onClick={() => setCreating(true)}><Plus /> Nova solicitação</button></header><section className="panel inventory-panel"><div className="table-scroll"><table><thead><tr><th>ID</th><th>Data</th><th>Destino</th><th>Solicitante</th><th>Prioridade</th><th>Produtos e andamento</th><th>Situação</th><th>Ações</th></tr></thead><tbody>{requests.data?.map((request) => <tr key={request.id}><td className="mono">{request.id.slice(0, 8)}</td><td>{formatDateTime(request.requestedAt)}</td><td>{request.destinationLocation}</td><td>{request.requester}</td><td>{priorityLabel(request.priority)}</td><td>{request.items.map((item) => `${item.productName} (${item.requestedQuantity})`).join(", ")}<small className="table-subline">{progressLabel(request)} · {request.justification}</small></td><td><span className={`request-status request-status--${request.status}`}>{statusLabel(request.status)}</span></td><td><div className="request-actions">{canReview && request.status === "pending" ? <button className="button button--compact button--secondary" onClick={() => setReviewing(request)}>Analisar</button> : null}{canReview && canReceivePurchase(request) ? <button className="button button--compact button--secondary" onClick={() => setReceiving(request)}>Receber compra</button> : null}{canReview && canDispatchTransfer(request) ? <button className="button button--compact button--secondary" onClick={() => setDispatching(request)}>Despachar transferência</button> : null}{request.transferShipments.filter(isOpenShipment).map((shipment) => <button key={shipment.id} className="button button--compact button--primary" onClick={() => setReceivingTransfer(shipment)}>Confirmar recebimento {shipment.reference}</button>)}{request.status === "fulfilled" ? "Concluída" : null}</div></td></tr>)}</tbody></table></div>{requests.isPending ? <div className="page-state">Carregando solicitações…</div> : null}{requests.isError ? <div className="page-state page-state--error">Não foi possível carregar as solicitações.</div> : null}{!requests.isPending && requests.data?.length === 0 ? <div className="page-state">Nenhuma reposição solicitada.</div> : null}</section><CreateDialog open={creating} onClose={() => setCreating(false)} /><ReviewDialog request={reviewing} onClose={() => setReviewing(null)} />{receiving ? <CreateMovementDialog key={receiving.id} open role={role} replenishmentRequest={receiving} onClose={() => setReceiving(null)} /> : null}<DispatchTransferDialog request={dispatching} onClose={() => setDispatching(null)} /><ReceiveTransferDialog shipment={receivingTransfer} onClose={() => setReceivingTransfer(null)} /></div>;
}

function CreateDialog({ open, onClose }: { open: boolean; onClose(): void }) {
  const locations = useLocations(); const inventory = useInventory(""); const create = useCreateReplenishmentRequest();
  const [destinationLocationId, setDestination] = useState(""); const [priority, setPriority] = useState<ReplenishmentPriority>("normal"); const [justification, setJustification] = useState(""); const [items, setItems] = useState([{ productId: "", quantity: 1 }]); const [error, setError] = useState("");
  if (!open) return null;
  async function submit(event: FormEvent) { event.preventDefault(); try { setError(""); await create.mutateAsync({ destinationLocationId, priority, justification, items }); onClose(); setJustification(""); setItems([{ productId: "", quantity: 1 }]); } catch (reason) { setError(errorText(reason)); } }
  return <div className="dialog-backdrop"><section className="dialog" role="dialog" aria-modal="true"><header><div><p className="path-label">ABASTECIMENTO</p><h2>Solicitar reposição</h2></div><button className="icon-button" onClick={onClose}><X /></button></header><form onSubmit={submit}><div className="form-grid"><label>Unidade de destino<select required value={destinationLocationId} onChange={(event) => setDestination(event.target.value)}><option value="">Selecione</option>{locations.data?.map((location) => <option key={location.id} value={location.id}>{location.name}</option>)}</select></label><label>Prioridade<select value={priority} onChange={(event) => setPriority(event.target.value as ReplenishmentPriority)}><option value="low">Baixa</option><option value="normal">Normal</option><option value="high">Alta</option><option value="urgent">Urgente</option></select></label></div><div className="form-grid form-grid--single">{items.map((item, index) => <div className="form-grid" key={index}><label>Produto<select required value={item.productId} onChange={(event) => setItems((current) => current.map((entry, position) => position === index ? { ...entry, productId: event.target.value } : entry))}><option value="">Selecione</option>{inventory.data?.map((product) => <option key={product.id} value={product.id}>{product.name} · {product.sku}</option>)}</select></label><label>Quantidade<input required type="number" min={1} max={1000000} value={item.quantity} onChange={(event) => setItems((current) => current.map((entry, position) => position === index ? { ...entry, quantity: Number(event.target.value) } : entry))} /></label>{items.length > 1 ? <button className="button button--secondary button--compact" type="button" onClick={() => setItems((current) => current.filter((_, position) => position !== index))}>Remover</button> : null}</div>)}<button className="button button--secondary button--compact" type="button" onClick={() => setItems((current) => [...current, { productId: "", quantity: 1 }])}>Adicionar produto</button><label>Justificativa<textarea required minLength={10} maxLength={1000} value={justification} onChange={(event) => setJustification(event.target.value)} /></label></div>{error ? <p className="form-error">{error}</p> : null}<footer><button type="button" className="button button--secondary" onClick={onClose}>Cancelar</button><button className="button button--primary" disabled={create.isPending}>Registrar solicitação</button></footer></form></section></div>;
}

function ReviewDialog({ request, onClose }: { request: ReplenishmentRequest | null; onClose(): void }) {
  if (!request) return null;
  return <ReviewDialogContent key={request.id} request={request} onClose={onClose} />;
}
function ReviewDialogContent({ request, onClose }: { request: ReplenishmentRequest; onClose(): void }) {
  const locations = useLocations(); const review = useReviewReplenishmentRequest(); const [note, setNote] = useState(""); const [error, setError] = useState("");
  const [decisions, setDecisions] = useState(() => request.items.map((item) => ({ itemId: item.id, transferQuantity: 0, purchaseQuantity: 0, sourceLocationId: null as string | null, purchaseReference: null as string | null })));
  async function submit(event: FormEvent) { event.preventDefault(); try { setError(""); await review.mutateAsync({ id: request.id, version: request.version, note, items: decisions }); onClose(); } catch (reason) { setError(errorText(reason)); } }
  function patch(index: number, value: Partial<(typeof decisions)[number]>) { setDecisions((current) => current.map((entry, position) => position === index ? { ...entry, ...value } : entry)); }
  return <div className="dialog-backdrop"><section className="dialog" role="dialog" aria-modal="true"><header><div><p className="path-label">ANÁLISE {request.id.slice(0, 8)}</p><h2>Decidir atendimento</h2></div><button className="icon-button" onClick={onClose}><X /></button></header><form onSubmit={submit}><p>{request.justification}</p><div className="form-grid form-grid--single">{request.items.map((item, index) => <fieldset key={item.id}><legend>{item.productName} · solicitado {item.requestedQuantity} · saldo no pedido {item.stockSnapshot}</legend><div className="form-grid"><label>Transferir<input type="number" min={0} max={item.requestedQuantity} value={decisions[index]!.transferQuantity} onChange={(event) => patch(index, { transferQuantity: Number(event.target.value) })} /></label><label>Comprar<input type="number" min={0} max={item.requestedQuantity} value={decisions[index]!.purchaseQuantity} onChange={(event) => patch(index, { purchaseQuantity: Number(event.target.value) })} /></label><label>Origem<select disabled={decisions[index]!.transferQuantity === 0} value={decisions[index]!.sourceLocationId ?? ""} onChange={(event) => patch(index, { sourceLocationId: event.target.value || null })}><option value="">Selecione</option>{locations.data?.filter((location) => location.id !== request.destinationLocationId).map((location) => <option key={location.id} value={location.id}>{location.name}</option>)}</select></label><label>Referência de compra<input maxLength={120} value={decisions[index]!.purchaseReference ?? ""} onChange={(event) => patch(index, { purchaseReference: event.target.value || null })} /></label></div>{item.trackingType === "serialized" ? <small>O sistema reservará automaticamente os patrimônios disponíveis; a seleção final ocorre no despacho.</small> : null}</fieldset>)}<label>Parecer do gestor<textarea required minLength={10} maxLength={1000} value={note} onChange={(event) => setNote(event.target.value)} /></label></div>{error ? <p className="form-error">{error}</p> : null}<footer><button type="button" className="button button--secondary" onClick={onClose}>Cancelar</button><button className="button button--primary" disabled={review.isPending}>Registrar decisão</button></footer></form></section></div>;
}

function DispatchTransferDialog({ request, onClose }: { request: ReplenishmentRequest | null; onClose(): void }) {
  if (!request) return null;
  return <DispatchTransferDialogContent key={request.id} request={request} onClose={onClose} />;
}

function DispatchTransferDialogContent({ request, onClose }: { request: ReplenishmentRequest; onClose(): void }) {
  const dispatch = useDispatchReplenishmentTransfer();
  const transferable = request.items.filter((item) => remainingTransfer(item) > 0 && item.sourceLocationId);
  const sources = Array.from(new Map(transferable.map((item) => [item.sourceLocationId!, item.sourceLocation ?? "Unidade de origem"])).entries());
  const [sourceId, setSourceId] = useState(sources[0]?.[0] ?? "");
  const [reference, setReference] = useState("");
  const [note, setNote] = useState("");
  const [quantities, setQuantities] = useState<Record<string, number>>(() => Object.fromEntries(transferable.map((item) => [item.id, item.trackingType === "quantity" ? remainingTransfer(item) : 0])));
  const [selectedAssets, setSelectedAssets] = useState<Record<string, boolean>>(() => Object.fromEntries(transferable.flatMap((item) => item.reservedAssets.slice(0, remainingTransfer(item)).map((asset) => [asset.id, true]))));
  const [error, setError] = useState("");
  const visibleItems = transferable.filter((item) => item.sourceLocationId === sourceId);
  async function submit(event: FormEvent) {
    event.preventDefault();
    const items = visibleItems.map((item) => {
      const assetIds = item.reservedAssets.filter((asset) => selectedAssets[asset.id]).map((asset) => asset.id);
      return { requestItemId: item.id, quantity: item.trackingType === "serialized" ? assetIds.length : quantities[item.id] ?? 0, assetIds };
    }).filter((item) => item.quantity > 0);
    try {
      setError("");
      await dispatch.mutateAsync({ requestId: request.id, reference, note, items });
      onClose();
    } catch (reason) { setError(errorText(reason)); }
  }
  return <div className="dialog-backdrop"><section className="dialog" role="dialog" aria-modal="true"><header><div><p className="path-label">TRANSFERÊNCIA {request.id.slice(0, 8)} &gt; DESPACHO</p><h2>Despachar itens reservados</h2></div><button className="icon-button" type="button" onClick={onClose} aria-label="Fechar"><X /></button></header><form onSubmit={submit}><div className="receipt-summary"><strong>Destino: {request.destinationLocation}</strong><span>O saldo sairá da origem e permanecerá em trânsito até a confirmação do destino.</span></div><div className="form-grid"><label>Unidade de origem<select required value={sourceId} onChange={(event) => setSourceId(event.target.value)}>{sources.map(([id, name]) => <option key={id} value={id}>{name}</option>)}</select></label><label>Documento / referência<input required maxLength={120} value={reference} onChange={(event) => setReference(event.target.value)} placeholder="Ex.: GUIA-2026-014" /></label></div><div className="form-grid form-grid--single">{visibleItems.map((item) => <fieldset key={item.id}><legend>{item.productName} · pendente {remainingTransfer(item)}</legend>{item.trackingType === "quantity" ? <label>Quantidade a despachar<input type="number" min={0} max={remainingTransfer(item)} value={quantities[item.id] ?? 0} onChange={(event) => setQuantities((current) => ({ ...current, [item.id]: Number(event.target.value) }))} /></label> : <div className="transfer-assets"><small>Selecione os patrimônios que sairão neste despacho.</small>{item.reservedAssets.map((asset) => <label className="check-row" key={asset.id}><input type="checkbox" checked={selectedAssets[asset.id] ?? false} onChange={(event) => setSelectedAssets((current) => ({ ...current, [asset.id]: event.target.checked }))} /> <span>{asset.assetTag}{asset.serialNumber ? ` · Série ${asset.serialNumber}` : ""}</span></label>)}</div>}</fieldset>)}<label>Observação do despacho<textarea maxLength={1000} value={note} onChange={(event) => setNote(event.target.value)} /></label></div>{error ? <p className="form-error">{error}</p> : null}<footer><button type="button" className="button button--secondary" onClick={onClose}>Cancelar</button><button className="button button--primary" disabled={dispatch.isPending || visibleItems.length === 0}>Confirmar despacho</button></footer></form></section></div>;
}

function ReceiveTransferDialog({ shipment, onClose }: { shipment: ReplenishmentTransferShipment | null; onClose(): void }) {
  if (!shipment) return null;
  return <ReceiveTransferDialogContent key={`${shipment.id}-${shipment.version}`} shipment={shipment} onClose={onClose} />;
}

function ReceiveTransferDialogContent({ shipment, onClose }: { shipment: ReplenishmentTransferShipment; onClose(): void }) {
  const receive = useReceiveReplenishmentTransfer();
  const pending = shipment.items.filter((item) => shipmentItemRemaining(item) > 0);
  const [reference, setReference] = useState("");
  const [note, setNote] = useState("");
  const [lines, setLines] = useState(() => Object.fromEntries(pending.map((item) => [item.id, { received: shipmentItemRemaining(item), rejected: 0, note: "" }])));
  const [error, setError] = useState("");
  function patch(id: string, value: Partial<{ received: number; rejected: number; note: string }>) { setLines((current) => ({ ...current, [id]: { ...current[id]!, ...value } })); }
  async function submit(event: FormEvent) {
    event.preventDefault();
    const items = pending.map((item) => ({ shipmentItemId: item.id, receivedQuantity: lines[item.id]?.received ?? 0, rejectedQuantity: lines[item.id]?.rejected ?? 0, note: lines[item.id]?.note ?? "" })).filter((item) => item.receivedQuantity + item.rejectedQuantity > 0);
    try {
      setError("");
      await receive.mutateAsync({ shipmentId: shipment.id, reference, note, version: shipment.version, items });
      onClose();
    } catch (reason) { setError(errorText(reason)); }
  }
  return <div className="dialog-backdrop"><section className="dialog" role="dialog" aria-modal="true"><header><div><p className="path-label">TRANSFERÊNCIA {shipment.reference} &gt; RECEBIMENTO</p><h2>Confirmar itens recebidos</h2></div><button className="icon-button" type="button" onClick={onClose} aria-label="Fechar"><X /></button></header><form onSubmit={submit}><div className="receipt-summary"><strong>{shipment.sourceLocation} → {shipment.destinationLocation}</strong><span>Informe separadamente o que chegou e o que foi recusado por divergência.</span></div><label>Comprovante / referência do recebimento<input required maxLength={120} value={reference} onChange={(event) => setReference(event.target.value)} placeholder="Ex.: RECEB-2026-014" /></label><div className="form-grid form-grid--single">{pending.map((item) => { const remaining = shipmentItemRemaining(item); return <fieldset key={item.id}><legend>{item.productName}{item.assetTag ? ` · ${item.assetTag}` : ""} · em trânsito {remaining}</legend><div className="form-grid"><label>Recebido<input type="number" min={0} max={remaining} value={lines[item.id]?.received ?? 0} onChange={(event) => patch(item.id, { received: Number(event.target.value) })} /></label><label>Recusado<input type="number" min={0} max={remaining} value={lines[item.id]?.rejected ?? 0} onChange={(event) => patch(item.id, { rejected: Number(event.target.value) })} /></label></div><label>Motivo da divergência<input maxLength={500} value={lines[item.id]?.note ?? ""} onChange={(event) => patch(item.id, { note: event.target.value })} placeholder="Obrigatório quando houver recusa" /></label></fieldset>; })}<label>Observação geral<textarea maxLength={1000} value={note} onChange={(event) => setNote(event.target.value)} /></label></div>{error ? <p className="form-error">{error}</p> : null}<footer><button type="button" className="button button--secondary" onClick={onClose}>Cancelar</button><button className="button button--primary" disabled={receive.isPending}>Registrar recebimento</button></footer></form></section></div>;
}

function remainingTransfer(item: ReplenishmentRequest["items"][number]) { return item.transferQuantity - item.transferReceivedQuantity - item.transferInTransitQuantity; }
function shipmentItemRemaining(item: ReplenishmentTransferShipment["items"][number]) { return item.quantity - item.receivedQuantity - item.rejectedQuantity; }
function isOpenShipment(shipment: ReplenishmentTransferShipment) { return shipment.status === "dispatched" || shipment.status === "partially_received"; }
function canReceivePurchase(request: ReplenishmentRequest) { return ["approved", "partially_approved", "in_fulfillment"].includes(request.status) && request.items.some((item) => item.receivedQuantity < item.purchaseQuantity); }
function canDispatchTransfer(request: ReplenishmentRequest) { return ["approved", "partially_approved", "in_fulfillment"].includes(request.status) && request.items.some((item) => remainingTransfer(item) > 0); }
function progressLabel(request: ReplenishmentRequest) {
  const purchase = request.items.reduce((total, item) => total + item.receivedQuantity, 0);
  const purchaseTotal = request.items.reduce((total, item) => total + item.purchaseQuantity, 0);
  const transferred = request.items.reduce((total, item) => total + item.transferReceivedQuantity, 0);
  const transferTotal = request.items.reduce((total, item) => total + item.transferQuantity, 0);
  const inTransit = request.items.reduce((total, item) => total + item.transferInTransitQuantity, 0);
  return `Compra ${purchase}/${purchaseTotal} · Transferência ${transferred}/${transferTotal} · Em trânsito ${inTransit}`;
}

function priorityLabel(priority: ReplenishmentPriority) { return ({ low: "Baixa", normal: "Normal", high: "Alta", urgent: "Urgente" } as const)[priority]; }
function statusLabel(status: ReplenishmentRequest["status"]) { return ({ pending: "Pendente", approved: "Aprovada", partially_approved: "Parcial", rejected: "Recusada", in_fulfillment: "Em atendimento", fulfilled: "Concluída", cancelled: "Cancelada" } as const)[status]; }
function errorText(reason: unknown) { return reason instanceof Error ? reason.message : String(reason); }
