import AlertTriangle from "lucide-react/dist/esm/icons/triangle-alert.mjs";
import CheckCircle from "lucide-react/dist/esm/icons/circle-check.mjs";
import FileSpreadsheet from "lucide-react/dist/esm/icons/file-spreadsheet.mjs";
import Upload from "lucide-react/dist/esm/icons/upload.mjs";
import X from "lucide-react/dist/esm/icons/x.mjs";
import { useState, type ChangeEvent } from "react";
import { canConfirmImport, type ImportResult } from "../../domain/import";
import { useLocations } from "../inventory/use-inventory";
import { discardImport, useConfirmImport, usePreviewImport } from "./use-import";

type Props = { open: boolean; onClose: () => void; onImported: (result: ImportResult) => void };

export function ImportSpreadsheetDialog({ open, onClose, onImported }: Props) {
  const [locationId, setLocationId] = useState("");
  const [error, setError] = useState("");
  const locations = useLocations();
  const preview = usePreviewImport();
  const confirm = useConfirmImport();

  if (!open) return null;

  async function handleFile(event: ChangeEvent<HTMLInputElement>) {
    const file = event.target.files?.[0];
    event.target.value = "";
    if (!file) return;
    if (!locationId) { setError("Selecione a unidade de destino antes do arquivo"); return; }
    if (file.size > 5 * 1024 * 1024) { setError("O arquivo deve ter no máximo 5 MB"); return; }
    try {
      setError("");
      preview.reset();
      const bytes = Array.from(new Uint8Array(await file.arrayBuffer()));
      await preview.mutateAsync({ fileName: file.name, bytes, locationId });
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    }
  }

  function close() {
    if (preview.data?.token) void discardImport(preview.data.token);
    onClose();
  }

  async function handleConfirm() {
    if (!preview.data || !canConfirmImport(preview.data) || !preview.data.token) return;
    try {
      setError("");
      const result = await confirm.mutateAsync(preview.data.token);
      onImported(result);
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    }
  }

  const data = preview.data;
  return (
    <div className="dialog-backdrop" role="presentation">
      <section className="dialog import-dialog" role="dialog" aria-modal="true" aria-labelledby="import-title">
        <header><div><p className="path-label">C:\ IMPORTAÇÃO SEGURA</p><h2 id="import-title">Importar planilha</h2></div><button className="icon-button" onClick={close} aria-label="Fechar importação" disabled={confirm.isPending}><X /></button></header>
        <div className="import-dialog__body">
          <div className="import-file-step">
            <label><span>Unidade de destino</span><select value={locationId} onChange={(event) => { setLocationId(event.target.value); preview.reset(); }} disabled={preview.isPending || confirm.isPending}><option value="">Selecione…</option>{locations.data?.map((location) => <option key={location.id} value={location.id}>{location.code} — {location.name}</option>)}</select></label>
            <label className={`import-dropzone ${!locationId ? "import-dropzone--disabled" : ""}`}><Upload /><strong>{preview.isPending ? "Analisando arquivo…" : "Selecionar planilha"}</strong><span>XLSX, XLS, XLSB, ODS ou CSV · até 5 MB</span><input type="file" accept=".xlsx,.xls,.xlsb,.ods,.csv" onChange={(event) => void handleFile(event)} disabled={!locationId || preview.isPending || confirm.isPending} /></label>
          </div>
          {error ? <div className="inline-error" role="alert">{error}</div> : null}
          {data ? <>
            <div className="import-summary" aria-label="Resumo da prévia"><div><span>Linhas</span><strong>{data.totalRows}</strong></div><div><span>Produtos</span><strong>{data.productsToCreate}</strong></div><div><span>Ativos</span><strong>{data.assetsToCreate}</strong></div><div className={data.errorRows ? "has-error" : "is-valid"}><span>Erros</span><strong>{data.errorRows}</strong></div></div>
            <div className={`import-verdict ${canConfirmImport(data) ? "import-verdict--valid" : "import-verdict--invalid"}`}>{canConfirmImport(data) ? <CheckCircle /> : <AlertTriangle />}<div><strong>{canConfirmImport(data) ? "Prévia pronta para confirmar" : "Corrija o arquivo antes de importar"}</strong><span>{data.sourceMode === "legacy" ? "Formato legado reconhecido automaticamente." : "Formato tabular reconhecido."} Nenhum dado foi gravado ainda.</span></div></div>
            {data.generalErrors.map((message) => <div className="inline-error" role="alert" key={message}>{message}</div>)}
            <div className="table-scroll import-preview-table"><table><thead><tr><th>Linha</th><th>Aba</th><th>SKU / produto</th><th>Controle</th><th>Qtd.</th><th>Patrimônio</th><th>Validação</th></tr></thead><tbody>{data.rows.map((row) => <tr key={`${row.sheet}-${row.rowNumber}`} className={row.errors.length ? "row-error" : ""}><td>{row.rowNumber}</td><td>{row.sheet}</td><td><strong>{row.sku}</strong><span className="table-subline">{row.name} · {row.category}</span></td><td>{row.trackingType === "serialized" ? "Serializado" : "Quantidade"}</td><td>{row.quantity}</td><td>{row.assetTag ?? "—"}</td><td>{row.errors.length ? row.errors.join(" · ") : <span className="import-row-valid">Válida</span>}</td></tr>)}</tbody></table></div>
          </> : null}
        </div>
        <footer className="dialog-footer"><button className="button button--secondary" type="button" onClick={close} disabled={confirm.isPending}>Cancelar</button><button className="button button--primary" type="button" onClick={() => void handleConfirm()} disabled={!data || !canConfirmImport(data) || confirm.isPending}><FileSpreadsheet /> {confirm.isPending ? "Importando…" : "Confirmar importação"}</button></footer>
      </section>
    </div>
  );
}
