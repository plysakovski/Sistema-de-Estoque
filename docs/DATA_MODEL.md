# Modelo de dados

| Tabela | Responsabilidade |
|---|---|
| `users` | Identidade local e papel de acesso. |
| `locations` | Unidades e locais físicos. |
| `categories` | Classificação normalizada. |
| `products` | Catálogo, SKU, categoria, mínimo e modelo de controle. |
| `stock_balances` | Saldo de produtos quantitativos por unidade. |
| `assets` | Exemplares serializados, patrimônio, série, unidade e estado. |
| `stock_movements` | Livro comum de entradas, saídas, ajustes e transferências. |
| `movement_batches` | Documento e agrupamento transacional de movimentações. |
| `replenishment_requests` | Cabeçalho, destino, prioridade e decisão da reposição. |
| `replenishment_request_items` | Quantidades solicitadas, aprovadas, compradas, transferidas, em trânsito, recebidas e recusadas. |
| `replenishment_transfer_shipments` | Despacho entre origem e destino, referências, situação e responsáveis. |
| `replenishment_transfer_shipment_items` | Linhas quantitativas ou patrimônios individuais e sua conferência. |
| `replenishment_transfer_asset_reservations` | Reserva exclusiva de ativos serializados antes do despacho. |
| `items` e `movements` | Estruturas legadas preservadas para migração conservadora. |
| `audit_logs` | Quem realizou cada alteração e instantâneos antes/depois. |
| `app_settings` | Preferências versionáveis. |
| `schema_migrations` | Versão do banco aplicada. |

## Regras invariantes

- Quantidades nunca podem ser negativas.
- SKU, patrimônio, série, código de unidade e código de categoria são únicos sem diferenciar maiúsculas.
- Categorias e unidades em uso não podem ser removidas fisicamente.
- Movimentações sempre apontam para um produto válido e, quando necessário, para um ativo válido.
- Exclusão funcional usa status/inativação; dados históricos permanecem íntegros.
- Um patrimônio pode pertencer a somente uma reserva de transferência e a no máximo um despacho em trânsito.
- A soma recebida e recusada nunca excede o que foi despachado.
- O destino só é creditado no recebimento; recusas recompõem o saldo e a reserva da origem.

O modelo híbrido já separa catálogo, ativos e saldos. Importações não possuem tabela temporária: a prévia fica somente em memória e o lote confirmado entra nas tabelas definitivas dentro de uma transação SQLite. A migração `0006_two_step_transfers.sql` introduz o livro de despachos sem apagar ou reinterpretar o histórico anterior.
