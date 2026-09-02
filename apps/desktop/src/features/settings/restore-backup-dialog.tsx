import AlertTriangle from "lucide-react/dist/esm/icons/triangle-alert.mjs";
import X from "lucide-react/dist/esm/icons/x.mjs";
import { useState, type FormEvent } from "react";
import type { BackupRecord, RestoreResult } from "../../domain/settings";
import { useRestoreBackup } from "./use-settings";

interface RestoreBackupDialogProps {
  backup: BackupRecord | null;
  onClose(): void;
  onRestored(result: RestoreResult): void;
}

export function RestoreBackupDialog({ backup, onClose, onRestored }: RestoreBackupDialogProps) {
  const [confirmation, setConfirmation] = useState("");
  const [error, setError] = useState("");
  const restore = useRestoreBackup();
  if (!backup) return null;

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!backup || confirmation !== "RESTAURAR") return;
    try {
      setError("");
      const result = await restore.mutateAsync({ fileName: backup.fileName });
      onRestored(result);
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    }
  }

  return (
    <div className="dialog-backdrop" role="presentation" onMouseDown={(event) => { if (event.target === event.currentTarget) onClose(); }}>
      <section className="dialog dialog--compact" role="dialog" aria-modal="true" aria-labelledby="restore-title">
        <header><div><p className="path-label">C:\ CONFIGURAÇÕES &gt; RESTAURAR</p><h2 id="restore-title">Restaurar backup</h2></div><button className="icon-button" type="button" onClick={onClose} aria-label="Fechar"><X /></button></header>
        <form onSubmit={handleSubmit}>
          <div className="restore-warning"><AlertTriangle aria-hidden="true" /><div><strong>O estado atual será substituído</strong><p>Antes da restauração, o sistema criará automaticamente um backup de emergência. O arquivo selecionado será validado novamente.</p></div></div>
          <div className="restore-target"><span>Backup selecionado</span><strong className="mono">{backup.fileName}</strong></div>
          <label className="restore-confirmation"><span>Digite <strong>RESTAURAR</strong> para confirmar</span><input autoFocus value={confirmation} onChange={(event) => setConfirmation(event.target.value)} autoComplete="off" /></label>
          {error ? <p className="form-error" role="alert">{error}</p> : null}
          <footer><button type="button" className="button button--secondary" onClick={onClose}>Cancelar</button><button type="submit" className="button button--danger" disabled={confirmation !== "RESTAURAR" || restore.isPending}>{restore.isPending ? "Restaurando…" : "Restaurar banco"}</button></footer>
        </form>
      </section>
    </div>
  );
}
