# Revisão de segurança — 1.8.0

Data: 31 de agosto de 2026

## Escopo

- Movimentações com múltiplos produtos.
- Recebimento de compras vinculado à reposição.
- Individualização de ativos por patrimônio e série.
- Ocorrências de defeito e decisões de manutenção.
- Separação entre catálogo e saldo físico.

## Controles aplicados

- O lote inteiro é executado em uma única transação SQLite; qualquer erro reverte todas as linhas.
- O backend limita lotes a 200 linhas, valida tipo uniforme, escopo de unidade e perfil do usuário.
- Operadores registram somente lotes de saída e ocorrências em unidades autorizadas.
- Recebimentos de reposição exigem gestor, solicitação aprovada, destino correto e quantidade dentro da compra aprovada.
- Patrimônios e números de série continuam protegidos por índices únicos.
- Produtos com política de série obrigatória rejeitam ativos sem série.
- Ativos recebidos preservam o ID do lote de origem.
- A ocorrência impede duplicidade aberta, coloca o ativo em manutenção e registra usuário e dados anteriores/posteriores.
- Todas as decisões relevantes geram auditoria no backend; controles visuais não são usados como autorização.

## Validação

- 34 testes Rust aprovados, incluindo atomicidade do lote, série obrigatória, vínculo de origem e ciclo de ocorrência.
- 14 testes TypeScript aprovados.
- TypeScript estrito e ESLint aprovados.
- Fluxos renderizados de lote, ocorrência e catálogo verificados sem erros ou alertas no console.
- `pnpm audit --prod`: nenhuma vulnerabilidade conhecida.
- `cargo audit`: nenhuma vulnerabilidade bloqueante; permanecem 17 avisos transitivos já documentados, incluindo dependências GTK não usadas no pacote Windows e bibliotecas trazidas pelo Tauri.

## Artefato Windows verificado

- Instalador: `release/StockManager Pro_1.8.0_x64-setup.exe`.
- Tamanho: 4.989.103 bytes.
- SHA-256: `4713739F622550BE96A4F0FB5220D4645D94F0A89A433F20F61F068CFA2032D1`.
- Subsistema PE: `2` (aplicação gráfica do Windows, sem terminal auxiliar).
- Authenticode: `NotSigned`, mantido como pendência explícita para a distribuição institucional.

## Pendências conhecidas

- Assinatura Authenticode continua necessária para distribuição institucional.
- O atendimento por transferência ainda precisa do fluxo separado de despacho e recebimento.
- Integração com diretório corporativo de colaboradores pode substituir futuramente o nome livre do responsável anterior.
