import KeyRound from "lucide-react/dist/esm/icons/key-round.mjs";
import ShieldCheck from "lucide-react/dist/esm/icons/shield-check.mjs";
import { useState, type FormEvent } from "react";

interface AuthScreenProps {
  setupRequired: boolean;
  busy: boolean;
  error: string | null;
  onSubmit(input: { username: string; displayName: string; password: string }): void;
}

export function AuthScreen({ setupRequired, busy, error, onSubmit }: AuthScreenProps) {
  const [username, setUsername] = useState("admin");
  const [displayName, setDisplayName] = useState("");
  const [password, setPassword] = useState("");

  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    onSubmit({ username, displayName, password });
  }

  return (
    <main className="auth-page">
      <section className="auth-card" aria-labelledby="auth-title">
        <div className="auth-brand"><span className="brand-mark">›_</span><strong>StockManager Pro</strong></div>
        <div className="auth-icon">{setupRequired ? <ShieldCheck aria-hidden="true" /> : <KeyRound aria-hidden="true" />}</div>
        <p className="path-label">SEGURANÇA / {setupRequired ? "ATIVAÇÃO" : "AUTENTICAÇÃO"}</p>
        <h1 id="auth-title">{setupRequired ? "Proteja o primeiro acesso" : "Entre para continuar"}</h1>
        <p className="auth-copy">{setupRequired ? "Crie a conta administradora responsável pela configuração e pelos demais usuários." : "Use sua credencial individual. Todas as ações sensíveis serão vinculadas à sua conta."}</p>
        <form onSubmit={submit}>
          {setupRequired ? <label>Nome do responsável<input autoComplete="name" required minLength={3} maxLength={100} value={displayName} onChange={(event) => setDisplayName(event.target.value)} autoFocus /></label> : null}
          <label>Usuário<input autoComplete="username" required minLength={3} maxLength={64} value={username} onChange={(event) => setUsername(event.target.value)} autoFocus={!setupRequired} /></label>
          <label>Senha<input type="password" autoComplete={setupRequired ? "new-password" : "current-password"} required minLength={12} maxLength={128} value={password} onChange={(event) => setPassword(event.target.value)} /></label>
          {setupRequired ? <small>Use uma frase-senha exclusiva com pelo menos 12 caracteres. Não inclua o nome do usuário.</small> : null}
          {error ? <p className="inline-error" role="alert">{error}</p> : null}
          <button className="button button--primary" type="submit" disabled={busy}>{busy ? "Verificando…" : setupRequired ? "Criar administrador" : "Entrar com segurança"}</button>
        </form>
        <footer><ShieldCheck aria-hidden="true" /> Sessão protegida e autorização validada pelo backend</footer>
      </section>
    </main>
  );
}
