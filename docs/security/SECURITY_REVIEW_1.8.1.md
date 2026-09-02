# Revisão de segurança — 1.8.1

Data: 1º de setembro de 2026

## Escopo

- Recebimento de compras vinculado a solicitações de reposição.
- Pré-carregamento dos produtos aprovados e do saldo ainda pendente.
- Recebimento parcial de ativos serializados e produtos por quantidade.
- Atualização do progresso da solicitação após cada lote.

## Controles aplicados

- O cliente não permite selecionar, trocar, adicionar ou remover produtos fora da solicitação aprovada.
- A quantidade máxima apresentada é calculada por `compra aprovada - quantidade já recebida`.
- Ativos serializados geram uma linha individual para cada unidade pendente, preservando patrimônio e número de série.
- Uma unidade não recebida deve ser desmarcada explicitamente; seus campos ficam desabilitados e ela não integra o lote.
- O backend continua sendo a autoridade: rejeita produto externo, excesso de quantidade, destino divergente e solicitações fora de situação válida.
- Recebimentos vinculados agora exigem nota fiscal ou referência do lote também no backend.
- O recebimento continua transacional: qualquer linha inválida reverte todo o lote.
- A lista de reposições é atualizada após o registro e expõe o progresso recebido/aprovado.

## Validação

- 35 testes Rust aprovados, incluindo referência obrigatória, recebimento parcial, saldo restante e conclusão integral.
- 16 testes TypeScript aprovados, incluindo geração de linhas por unidade serializada e limite do saldo pendente.
- TypeScript estrito, ESLint e build de produção aprovados.
- Fluxo visual verificado em `http://127.0.0.1:5173/`, sem erro ou alerta no console.
- `pnpm audit --prod`: nenhuma vulnerabilidade conhecida.
- `cargo audit`: nenhuma vulnerabilidade bloqueante; 17 avisos permitidos permanecem em dependências transitivas, principalmente GTK3 para plataformas não Windows e bibliotecas trazidas pelo Tauri.

## Artefato Windows verificado

- Instalador: `release/StockManager Pro_1.8.1_x64-setup.exe`.
- Tamanho: 4.992.837 bytes.
- SHA-256: `4F8F520535F4A21FF45AC34BCF26E6E0E1DBF5A330243CC2E536F2732E96DEED`.
- Executável principal: subsistema PE `2` (aplicação gráfica do Windows, sem terminal auxiliar).
- Authenticode: `NotSigned`, mantido como pendência explícita para distribuição institucional.

## Pendências conhecidas

- Assinatura Authenticode continua necessária para distribuição institucional.
- O atendimento por transferência permanece separado do recebimento de compras e ainda requer despacho e confirmação em duas etapas.
