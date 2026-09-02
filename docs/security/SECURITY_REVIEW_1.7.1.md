# Revisão de segurança — 1.7.1

Data: 31 de agosto de 2026

## Alteração analisada

Correção da autorização de transições operacionais de ativos para operadores vinculados à unidade do patrimônio.

## Regra aplicada no backend

- Operador pode executar somente `available → in_use` e `in_use → available`.
- O ativo precisa estar em uma unidade atribuída ao operador.
- A unidade de destino precisa ser igual à unidade atual.
- Transferência, manutenção, baixa e demais correções continuam bloqueadas.
- A operação mantém movimentação e auditoria com o usuário responsável.
- Gestor e administrador mantêm as permissões anteriores.

## Verificações obrigatórias

- Teste positivo da transição operacional em unidade autorizada.
- Teste negativo de tentativa de manutenção direta pelo operador.
- Testes completos Rust, TypeScript e análise estática.
- Repetição de `pnpm audit --prod` e `cargo audit` antes da distribuição.

## Resultado

A correção reduz privilégio excessivo sem impedir a rotina do operador. A autorização continua sendo validada no backend e não depende dos controles visuais da interface.

- 32 testes Rust aprovados.
- 14 testes TypeScript aprovados, incluindo as restrições visuais do formulário do operador.
- TypeScript estrito e ESLint aprovados.
- Verificação renderizada da página de ativos aprovada, sem erros ou alertas no console.
- `pnpm audit --prod`: nenhuma vulnerabilidade conhecida.
- `cargo audit`: nenhuma vulnerabilidade bloqueante e os mesmos 17 avisos transitivos documentados na revisão 1.7.0.

## Artefato Windows verificado

- Instalador: `StockManager Pro_1.7.1_x64-setup.exe`.
- SHA-256: `9F18B6BA2865DAA6BD28F75212AACAF000B06BE8448EE1D8D9A4A561E899C2B9`.
- Subsistema PE: Windows GUI (`2`), sem abertura automática de terminal.
- Authenticode: ainda não assinado; a assinatura permanece uma pendência de distribuição.
