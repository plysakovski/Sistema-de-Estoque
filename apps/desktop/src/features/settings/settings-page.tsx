import DatabaseBackup from "lucide-react/dist/esm/icons/database-backup.mjs";
import HardDrive from "lucide-react/dist/esm/icons/hard-drive.mjs";
import FileSpreadsheet from "lucide-react/dist/esm/icons/file-spreadsheet.mjs";
import Copy from "lucide-react/dist/esm/icons/copy.mjs";
import KeyRound from "lucide-react/dist/esm/icons/key-round.mjs";
import RotateCcw from "lucide-react/dist/esm/icons/rotate-ccw.mjs";
import ShieldCheck from "lucide-react/dist/esm/icons/shield-check.mjs";
import { useState } from "react";
import type { BackupRecord, RestoreResult } from "../../domain/settings";
import { formatDateTime } from "../../shared/format";
import { RestoreBackupDialog } from "./restore-backup-dialog";
import { ImportSpreadsheetDialog } from "./import-spreadsheet-dialog";
import { useAcknowledgeRecoveryKey, useBackups, useCreateBackup, useRecoveryKeyStatus, useRevealRecoveryKey } from "./use-settings";

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / 1024 / 1024).toFixed(2)} MB`;
}

export function SettingsPage({ isAdmin }: { isAdmin: boolean }) {
  const [selectedBackup, setSelectedBackup] = useState<BackupRecord | null>(null);
  const [importOpen, setImportOpen] = useState(false);
  const [message, setMessage] = useState("");
  const [error, setError] = useState("");
  const backups = useBackups();
  const createBackup = useCreateBackup();
  const recoveryStatus = useRecoveryKeyStatus(isAdmin);
  const revealRecoveryKey = useRevealRecoveryKey();
  const acknowledgeRecoveryKey = useAcknowledgeRecoveryKey();
  const [recoveryPassword, setRecoveryPassword] = useState("");
  const [revealedKey, setRevealedKey] = useState("");

  async function handleCreateBackup() {
    try {
      setError("");
      const backup = await createBackup.mutateAsync();
      setMessage(`Backup criado com segurança: ${backup.fileName}`);
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    }
  }

  function handleRestored(result: RestoreResult) {
    setSelectedBackup(null);
    setMessage(`Banco restaurado de ${result.restoredFrom}. Cópia de emergência: ${result.safetyBackup}`);
  }

  async function handleRevealRecoveryKey() {
    try {
      setError("");
      const result = await revealRecoveryKey.mutateAsync(recoveryPassword);
      setRevealedKey(result.key);
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    }
  }

  async function handleAcknowledgeRecoveryKey() {
    try {
      setError("");
      await acknowledgeRecoveryKey.mutateAsync(recoveryPassword);
      setMessage("Confirmação registrada na auditoria. Mantenha a chave em um cofre externo e controlado.");
      setRevealedKey("");
      setRecoveryPassword("");
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    }
  }

  return (
    <div>
      <header className="page-heading"><div><p className="path-label">C:\ CONFIGURAÇÕES</p><h1>Dados e recuperação</h1><p>Importe planilhas com prévia e proteja o banco local com cópias verificadas.</p></div><div className="page-heading__actions"><button className="button button--secondary" onClick={() => setImportOpen(true)}><FileSpreadsheet aria-hidden="true" /> Importar planilha</button><button className="button button--primary" onClick={() => void handleCreateBackup()} disabled={createBackup.isPending}><DatabaseBackup aria-hidden="true" /> {createBackup.isPending ? "Criando…" : "Criar backup agora"}</button></div></header>
      {message ? <div className="success-message" role="status">{message}</div> : null}{error ? <div className="inline-error" role="alert">{error}</div> : null}
      <section className="settings-cards" aria-label="Características do armazenamento"><div><HardDrive /><span>Armazenamento</span><strong>SQLite criptografado</strong><small>SQLCipher e cofre do Windows</small></div><div><ShieldCheck /><span>Importação</span><strong>Prévia sem gravação</strong><small>Validação completa por linha</small></div><div><RotateCcw /><span>Recuperação</span><strong>Cópias criptografadas</strong><small>Criadas antes de restaurar ou importar</small></div></section>
      {isAdmin ? <section className="panel recovery-panel" aria-labelledby="recovery-title">
        <div><KeyRound aria-hidden="true" /><h2 id="recovery-title">Chave de recuperação</h2></div>
        <p>Guarde esta chave em um cofre externo. Ela é necessária para abrir o banco em outra máquina se o cofre de credenciais do Windows não estiver disponível.</p>
        <p className={recoveryStatus.data?.acknowledged ? "success-message" : "inline-error"}>{recoveryStatus.data?.acknowledged ? "Armazenamento externo confirmado pelo administrador." : "A chave ainda não teve seu armazenamento externo confirmado."}</p>
        <label>Confirme sua senha de administrador<input type="password" autoComplete="current-password" value={recoveryPassword} onChange={(event) => setRecoveryPassword(event.target.value)} /></label>
        <div className="page-heading__actions"><button className="button button--secondary" type="button" disabled={!recoveryPassword || revealRecoveryKey.isPending} onClick={() => void handleRevealRecoveryKey()}><KeyRound /> {revealRecoveryKey.isPending ? "Verificando…" : "Revelar chave"}</button></div>
        {revealedKey ? <div className="recovery-key"><code>{revealedKey}</code><button className="button button--secondary button--compact" type="button" onClick={() => void navigator.clipboard.writeText(revealedKey)}><Copy /> Copiar</button><p>Para recuperar em outra máquina, salve apenas essa sequência em <code>stockmanager.recovery-key</code> ao lado do banco. O sistema importará a chave para o cofre do Windows e apagará o arquivo temporário após validar o banco.</p><button className="button button--primary" type="button" disabled={acknowledgeRecoveryKey.isPending} onClick={() => void handleAcknowledgeRecoveryKey()}>{acknowledgeRecoveryKey.isPending ? "Registrando…" : "Confirmar armazenamento externo"}</button></div> : null}
      </section> : null}
      <section className="panel inventory-panel"><h2>&gt; Backups disponíveis</h2><div className="table-scroll"><table><thead><tr><th>Arquivo</th><th>Tipo</th><th>Criação</th><th>Tamanho</th><th>Integridade</th><th><span className="sr-only">Ações</span></th></tr></thead><tbody>{backups.data?.map((backup) => <tr key={backup.fileName}><td className="mono">{backup.fileName}</td><td>{backup.kind === "manual" ? "Manual" : backup.kind === "import" ? "Pré-importação" : "Pré-restauração"}</td><td>{formatDateTime(backup.createdAt)}</td><td>{formatBytes(backup.sizeBytes)}</td><td><span className={`backup-validity backup-validity--${backup.valid ? "valid" : "invalid"}`}>{backup.valid ? "Válido" : "Inválido"}</span></td><td>{isAdmin ? <button className="button button--secondary button--compact" type="button" disabled={!backup.valid} onClick={() => setSelectedBackup(backup)}><RotateCcw /> Restaurar</button> : <span className="muted">Somente administrador</span>}</td></tr>)}</tbody></table></div>{backups.isPending ? <div className="page-state">Carregando backups…</div> : null}{backups.isError ? <div className="page-state page-state--error">Não foi possível carregar os backups.</div> : null}{!backups.isPending && backups.data?.length === 0 ? <div className="page-state">Nenhum backup criado ainda.</div> : null}</section>
      <ImportSpreadsheetDialog key={importOpen ? "import-open" : "import-closed"} open={importOpen} onClose={() => setImportOpen(false)} onImported={(result) => { setImportOpen(false); setMessage(`${result.productsCreated} produtos e ${result.quantityImported} itens importados. Backup de segurança: ${result.safetyBackup}`); }} />
      <RestoreBackupDialog key={selectedBackup?.fileName ?? "restore-closed"} backup={selectedBackup} onClose={() => setSelectedBackup(null)} onRestored={handleRestored} />
    </div>
  );
}
