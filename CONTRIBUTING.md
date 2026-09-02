# Como contribuir

Obrigado pelo interesse no StockManager Pro. Alterações devem preservar as regras de negócio, o princípio de menor privilégio e a separação entre domínio, interface e infraestrutura.

## Fluxo recomendado

1. Crie uma branch a partir de `main`.
2. Mantenha cada alteração pequena e orientada a um único objetivo.
3. Inclua ou atualize testes para regras de negócio e permissões.
4. Execute as verificações locais.
5. Abra um pull request explicando problema, solução, riscos e validação.

```powershell
cd apps\desktop
pnpm.cmd install
pnpm.cmd check
cargo fmt --manifest-path src-tauri\Cargo.toml --all -- --check
cargo test --manifest-path src-tauri\Cargo.toml
```

## Critérios de aceitação

- A autorização deve ser validada pelo backend, nunca apenas pela interface.
- Escritas relacionadas devem ser atômicas e auditadas.
- Segredos, bancos reais, backups, logs e dados pessoais não podem entrar no Git.
- Migrações publicadas não devem ser reescritas; crie uma nova migração.
- Mudanças visuais relevantes devem incluir evidência antes/depois.
- Novas dependências precisam de justificativa e revisão de segurança.

Vulnerabilidades não devem ser abertas como issue pública. Siga [SECURITY.md](SECURITY.md).
