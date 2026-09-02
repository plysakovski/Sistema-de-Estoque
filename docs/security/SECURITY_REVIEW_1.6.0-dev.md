# Revisão de segurança — 1.6.0-dev

Data: 31/08/2026
Estado: **criptografia aprovada nos testes locais; distribuição pública ainda bloqueada por assinatura de código**

## Escopo revisado

- autenticação local e armazenamento de senhas;
- sessão e autorização de comandos Tauri;
- perfis operador, gestor e administrador;
- solicitações e aprovação de ajustes;
- trilha de auditoria e identidade dos atores;
- gestão de usuários e invalidação de sessões;
- dependências JavaScript e Rust;
- CSP, capacidades Tauri, logs e busca de segredos;
- banco SQLite, backups e migrações.

## Controles implementados

- Argon2id com 19 MiB, duas iterações, paralelismo 1 e salt aleatório por senha.
- Senhas limitadas a 12–128 caracteres, com Unicode aceito e sem truncamento silencioso.
- Resposta genérica de credenciais inválidas e verificação de hash fictício para usuários inexistentes.
- Bloqueio de 15 minutos após cinco tentativas malsucedidas.
- Sessão somente na memória Rust, com 15 minutos de inatividade e máximo absoluto de oito horas.
- Validação de sessão e permissão no backend em todos os comandos de negócio.
- Identidade real do usuário em movimentações e auditorias; o identificador fixo não é mais usado pelos comandos.
- Invalidação de sessões quando perfil, situação ou senha é alterado.
- Proteção contra remoção/rebaixamento do último administrador e contra autorremoção administrativa.
- Segregação entre solicitante e revisor de ajuste.
- Controle de concorrência por versão do alvo e da solicitação.
- Alteração, decisão, movimentação e auditoria na mesma transação SQLite.
- Dados de demonstração restritos a compilações de desenvolvimento.
- SQLCipher com chave aleatória de 256 bits e chave operacional no Credential Manager do Windows.
- Migração do SQLite legado para arquivo criptografado validado antes da substituição do original.
- Backups e cópias de segurança criptografados e rejeitados sem a chave correspondente.
- Recuperação controlada por arquivo temporário, validação criptográfica e importação para o cofre do Windows.
- Revelação da chave restrita ao administrador, condicionada à redigitação da senha e registrada em auditoria.
- Reautenticação sensível limitada a cinco falhas, com bloqueio, encerramento da sessão e auditoria.

## Resultados automatizados

| Verificação | Resultado |
|---|---|
| `cargo test` | 30 testes aprovados |
| `cargo clippy --all-targets -- -D warnings` | aprovado |
| ESLint | aprovado, zero avisos |
| Vitest | 12 testes aprovados |
| TypeScript + Vite produção | aprovado |
| `pnpm audit --prod` | nenhuma vulnerabilidade conhecida |
| `cargo audit` | nenhuma vulnerabilidade classificada como vulnerabilidade; 17 avisos de manutenção/solidez no lockfile |
| Busca local de segredos | nenhum segredo incorporado encontrado |

Os avisos RustSec relativos a GTK3/GLib pertencem às dependências condicionais Linux do Tauri e não integram a árvore do alvo `x86_64-pc-windows-msvc`. O aviso de solidez `RUSTSEC-2024-0429` não aparece na árvore Windows. Os demais avisos transitivos devem continuar sendo monitorados nas atualizações do Tauri.

## Artefato verificado

- Instalador: `release/StockManager Pro_1.6.0_x64-setup.exe`
- Tamanho: 4.925.605 bytes
- SHA-256: `7F922B9E47FB03BBE24CEF648C67B008D37201741FA81D073EA35360784734A0`
- Empacotamento NSIS: concluído com sucesso em Windows x64.
- Assinatura Authenticode: pendente conforme SEC-004; o hash garante integridade desta cópia, mas não substitui uma assinatura confiável.

## Achados e decisão

### SEC-001 — Criptografia do banco e dos backups

- Severidade original: **alta**
- Estado: corrigido e verificado localmente
- Evidência: arquivos novos e migrados não apresentam o cabeçalho SQLite; consultas sem chave falham; backups permanecem criptografados; restauração e integridade passaram nos testes automatizados.
- Implementação: SQLCipher, chave aleatória de 256 bits, Credential Manager, memória zerável, migração com arquivo temporário validado e recuperação administrativa auditada.
- Risco residual: cópias antigas em texto claro e remanência física em SSD devem ser tratadas pela organização com criptografia de volume e descarte seguro.

### SEC-002 — CSP permite estilos inline

- Severidade: média
- Estado: aceito temporariamente
- Evidência: `style-src 'self' 'unsafe-inline'` é necessário para estilos dinâmicos atuais da interface.
- Correção: remover estilos inline dinâmicos ou adotar uma estratégia compatível com hash/nonce antes da homologação de alto risco.

### SEC-003 — Autenticação de fator único

- Severidade: média para ambientes comuns; alta para determinados órgãos
- Estado: planejado
- Correção: suporte opcional a segundo fator e política institucional, sem enfraquecer o modo local/offline.

### SEC-004 — Atualização e assinatura do aplicativo

- Severidade: alta para distribuição externa
- Estado: planejado; bloqueia implantação pública
- Correção: assinatura de código do instalador e do executável, canal de atualização assinado e procedimento de rotação/revogação do certificado.

## Risco residual

A autenticação, o RBAC, o fluxo de aprovação e a criptografia estão adequados para homologação funcional local. A versão não deve ser distribuída a órgãos públicos ou produção sensível até que SEC-004 seja concluído e uma revisão independente valide a implementação criptográfica, a recuperação operacional e o instalador assinado.
