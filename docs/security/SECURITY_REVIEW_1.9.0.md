# Revisão de segurança — 1.9.0

Data: 3 de setembro de 2026

## Escopo

- Aprovação, reserva, despacho e recebimento de transferências de reposição.
- Produtos controlados por quantidade e ativos patrimoniais serializados.
- Recebimento parcial, recusa por divergência e novo despacho.
- Autorização por perfil e escopo de unidade.

## Controles aplicados

- Somente gestor ou administrador pode despachar, com acesso obrigatório à unidade de origem.
- Operador, gestor ou administrador pode receber somente quando vinculado à unidade de destino.
- O backend é a autoridade para produto, origem, destino, quantidade, patrimônio, situação e versão do registro.
- O saldo quantitativo é reservado na aprovação, debitado no despacho e creditado no destino apenas no recebimento.
- Patrimônios são reservados individualmente, não podem ser prometidos duas vezes e ficam bloqueados enquanto estão em trânsito.
- Movimentações comuns não consomem saldo reservado nem alteram patrimônio reservado ou em trânsito.
- Concorrência é protegida por transação SQLite e controle otimista de versão no recebimento.
- Recusa exige motivo, recompõe a origem e mantém o saldo pendente para novo despacho.
- Auditoria registra despacho e recebimento com solicitação, referência, responsáveis e quantidades aceitas ou recusadas.

## Validação

- 37 testes Rust aprovados, incluindo divergência quantitativa, reenvio e transferência de patrimônio específico.
- 16 testes TypeScript aprovados.
- TypeScript estrito, ESLint, build de produção e `cargo check` aprovados.
- Migração de banco aplicada com chaves estrangeiras, verificações de faixa e índices dedicados.
- `pnpm audit --prod`: nenhuma vulnerabilidade conhecida.
- `cargo audit`: nenhuma vulnerabilidade bloqueante; 17 avisos permitidos permanecem em dependências transitivas, principalmente GTK3 de plataformas não Windows e bibliotecas trazidas pelo Tauri.
- Fluxo visual completo validado da solicitação à conclusão, inclusive estado em trânsito, sem erros ou alertas no console.

## Artefato Windows verificado

- Instalador: `StockManager Pro_1.9.0_x64-setup.exe`.
- Tamanho: 5.012.850 bytes.
- SHA-256: `93902FB9420780AEA4A4E71073C092FE23F9E0430A0AE843659AF0185898233E`.
- Executável principal: subsistema PE `2` (aplicação gráfica do Windows, sem terminal auxiliar).
- Authenticode: `NotSigned`, mantido como pendência explícita para distribuição institucional.

## Riscos residuais

- A aplicação continua destinada a homologação controlada até a assinatura Authenticode e os controles institucionais descritos no estado do projeto.
- A confirmação de identidade física do recebedor depende do procedimento operacional da organização; a aplicação registra a conta autenticada.
- Uma política futura poderá exigir dupla conferência para transferências acima de valor ou criticidade configurável.
- Teste de invasão independente e ensaio de recuperação continuam obrigatórios antes de uso institucional.
