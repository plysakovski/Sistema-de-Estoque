import ClipboardCheck from "lucide-react/dist/esm/icons/clipboard-check.mjs";
import Plus from "lucide-react/dist/esm/icons/plus.mjs";
import X from "lucide-react/dist/esm/icons/x.mjs";
import { useState, type FormEvent } from "react";
import type { AdjustmentRequest } from "../../domain/adjustments";
import type { UserRole } from "../../domain/auth";
import { formatDateTime } from "../../shared/format";
import { useInventory, useLocations } from "../inventory/use-inventory";
import { useAssets } from "../movements/use-movements";
import { useAdjustments, useCreateAdjustment, useReviewAdjustment } from "./use-adjustments";

export function AdjustmentsPage({ role }: { role: UserRole }) {
  const [createOpen, setCreateOpen] = useState(false);
  const [reviewing, setReviewing] = useState<AdjustmentRequest | null>(null);
  const requests = useAdjustments();
  const canReview = role === "manager" || role === "admin";

  return <div>
    <header className="page-heading"><div><p className="path-label">C:\ SOLICITAÇÕES &gt; AJUSTES</p><h1>Solicitações de ajuste</h1><p>{canReview ? "Analise justificativas e evidências antes de autorizar qualquer alteração." : "Solicite correções sem alterar diretamente o estoque."}</p></div><button className="button button--primary" onClick={() => setCreateOpen(true)}><Plus aria-hidden="true" /> Solicitar ajuste</button></header>
    <section className="panel inventory-panel"><div className="table-scroll"><table><thead><tr><th>ID</th><th>Data</th><th>Solicitante</th><th>Produto / ativo</th><th>Alteração</th><th>Justificativa</th><th>Situação</th><th>Revisor</th>{canReview ? <th>Ação</th> : null}</tr></thead><tbody>{requests.data?.map((request) => <tr key={request.id}><td className="mono">{request.id.slice(0, 8)}</td><td>{formatDateTime(request.requestedAt)}</td><td>{request.requester}</td><td>{request.productName}<small className="table-subline">{request.assetTag ?? request.sku}</small></td><td>{requestSummary(request)}</td><td className="adjustment-description">{request.description}</td><td><span className={`request-status request-status--${request.status}`}>{statusLabel(request.status)}</span></td><td>{request.reviewer ?? "—"}</td>{canReview ? <td>{request.status === "pending" ? <button className="button button--compact button--secondary" onClick={() => setReviewing(request)}>Analisar</button> : "Concluída"}</td> : null}</tr>)}</tbody></table></div>{requests.isPending ? <div className="page-state">Carregando solicitações…</div> : null}{requests.isError ? <div className="page-state page-state--error">Não foi possível carregar as solicitações.</div> : null}{!requests.isPending && requests.data?.length === 0 ? <div className="page-state">Nenhuma solicitação registrada.</div> : null}</section>
    <CreateAdjustmentDialog open={createOpen} onClose={() => setCreateOpen(false)} />
    <ReviewAdjustmentDialog request={reviewing} onClose={() => setReviewing(null)} />
  </div>;
}

function CreateAdjustmentDialog({ open, onClose }: { open: boolean; onClose(): void }) {
  const [productId, setProductId] = useState("");
  const [assetId, setAssetId] = useState("");
  const [locationId, setLocationId] = useState("");
  const [quantity, setQuantity] = useState("0");
  const [requestedStatus, setRequestedStatus] = useState("");
  const [requestedLocationId, setRequestedLocationId] = useState("");
  const [description, setDescription] = useState("");
  const [error, setError] = useState("");
  const inventory = useInventory("");
  const locations = useLocations();
  const selectedProduct = inventory.data?.find((item) => item.id === productId);
  const serialized = selectedProduct?.trackingType === "serialized";
  const assets = useAssets(serialized ? productId : "");
  const create = useCreateAdjustment();
  if (!open) return null;

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!productId || description.trim().length < 10 || (serialized ? !assetId || (!requestedStatus && !requestedLocationId) : !locationId || Number(quantity) < 0)) {
      setError("Preencha o alvo, a alteração pretendida e uma justificativa com pelo menos 10 caracteres."); return;
    }
    try {
      setError("");
      await create.mutateAsync({ productId, assetId: serialized ? assetId : null, kind: serialized ? "asset_update" : "quantity_adjustment", locationId: serialized ? null : locationId, requestedQuantity: serialized ? null : Number(quantity), requestedStatus: serialized ? requestedStatus || null : null, requestedLocationId: serialized ? requestedLocationId || null : null, description: description.trim() });
      setProductId(""); setAssetId(""); setLocationId(""); setDescription(""); onClose();
    } catch (reason) { setError(errorText(reason)); }
  }

  return <div className="dialog-backdrop" role="presentation"><section className="dialog dialog--compact" role="dialog" aria-modal="true" aria-labelledby="create-adjustment-title"><header><div><p className="path-label">SOLICITAÇÃO CONTROLADA</p><h2 id="create-adjustment-title">Solicitar ajuste</h2></div><button className="icon-button" onClick={onClose} aria-label="Fechar"><X /></button></header><form onSubmit={submit}><div className="form-grid form-grid--single"><label>Produto<select required value={productId} onChange={(event) => { setProductId(event.target.value); setAssetId(""); }}><option value="">Selecione</option>{inventory.data?.map((item) => <option key={item.id} value={item.id}>{item.sku} — {item.name}</option>)}</select></label>{serialized ? <><label>Ativo<select required value={assetId} onChange={(event) => setAssetId(event.target.value)}><option value="">Selecione</option>{assets.data?.map((asset) => <option key={asset.id} value={asset.id}>{asset.assetTag} — {asset.location}</option>)}</select></label><label>Novo estado (opcional)<select value={requestedStatus} onChange={(event) => setRequestedStatus(event.target.value)}><option value="">Manter atual</option><option value="available">Disponível</option><option value="in_use">Em uso</option><option value="maintenance">Manutenção</option><option value="disposed">Baixado</option></select></label><label>Nova unidade (opcional)<select value={requestedLocationId} onChange={(event) => setRequestedLocationId(event.target.value)}><option value="">Manter atual</option>{locations.data?.map((location) => <option key={location.id} value={location.id}>{location.name}</option>)}</select></label></> : <><label>Unidade do saldo<select required value={locationId} onChange={(event) => setLocationId(event.target.value)}><option value="">Selecione</option>{locations.data?.map((location) => <option key={location.id} value={location.id}>{location.name}</option>)}</select></label><label>Nova quantidade<input type="number" min="0" required value={quantity} onChange={(event) => setQuantity(event.target.value)} /></label></>}<label>Justificativa<textarea rows={4} minLength={10} maxLength={1000} required value={description} onChange={(event) => setDescription(event.target.value)} placeholder="Descreva a divergência, a conferência realizada e as evidências disponíveis." /></label></div>{error ? <p className="form-error" role="alert">{error}</p> : null}<footer><button type="button" className="button button--secondary" onClick={onClose}>Cancelar</button><button type="submit" className="button button--primary" disabled={create.isPending}>{create.isPending ? "Registrando…" : "Registrar solicitação"}</button></footer></form></section></div>;
}

function ReviewAdjustmentDialog({ request, onClose }: { request: AdjustmentRequest | null; onClose(): void }) {
  const [note, setNote] = useState("");
  const [error, setError] = useState("");
  const review = useReviewAdjustment();
  if (!request) return null;
  const activeRequest = request;
  async function decide(decision: "approved" | "rejected") {
    if (note.trim().length < 5) { setError("Registre um parecer com pelo menos 5 caracteres."); return; }
    try { setError(""); await review.mutateAsync({ id: activeRequest.id, decision, note: note.trim(), version: activeRequest.version }); setNote(""); onClose(); } catch (reason) { setError(errorText(reason)); }
  }
  return <div className="dialog-backdrop" role="presentation"><section className="dialog dialog--compact" role="dialog" aria-modal="true" aria-labelledby="review-title"><header><div><p className="path-label">SEGREGAÇÃO DE FUNÇÕES</p><h2 id="review-title">Analisar solicitação</h2></div><button className="icon-button" onClick={onClose} aria-label="Fechar"><X /></button></header><form onSubmit={(event) => event.preventDefault()}><div className="review-summary"><ClipboardCheck /><strong>{request.productName}</strong><span>ID {request.id}</span><p>{requestSummary(request)}</p><p>{request.description}</p></div><div className="form-grid form-grid--single"><label>Parecer do gestor<textarea rows={4} minLength={5} maxLength={1000} value={note} onChange={(event) => setNote(event.target.value)} placeholder="Registre os critérios e evidências considerados na decisão." /></label></div>{error ? <p className="form-error" role="alert">{error}</p> : null}<footer><button type="button" className="button button--danger" disabled={review.isPending} onClick={() => void decide("rejected")}>Recusar</button><button type="button" className="button button--primary" disabled={review.isPending} onClick={() => void decide("approved")}>Autorizar ajuste</button></footer></form></section></div>;
}

function requestSummary(request: AdjustmentRequest) { return request.kind === "quantity_adjustment" ? `${request.location ?? "Unidade"}: definir saldo para ${request.requestedQuantity}` : `${request.assetTag ?? "Ativo"}: ${request.requestedLocation ?? "mesma unidade"}, ${request.requestedStatus ?? "mesmo estado"}`; }
function statusLabel(status: AdjustmentRequest["status"]) { return status === "pending" ? "Pendente" : status === "approved" ? "Autorizada" : status === "rejected" ? "Recusada" : "Cancelada"; }
function errorText(reason: unknown) { return reason instanceof Error ? reason.message : String(reason); }
