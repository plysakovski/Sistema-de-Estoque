# Arquitetura

## Objetivo

Manter a interface desacoplada da persistência e concentrar integridade, transações e acesso ao sistema operacional no núcleo Rust.

```text
React/TypeScript
  ├─ app: composição, navegação e providers
  ├─ features: telas e casos de uso por domínio
  ├─ domain: tipos, contratos e validação
  └─ infrastructure: adaptador Tauri e adaptador de demonstração
                         │ comandos tipados
                         ▼
Tauri/Rust
  ├─ commands: fronteira IPC mínima
  ├─ imports: leitura, normalização, validação e prévias temporárias
  ├─ database: transações e consultas
  ├─ models: contratos serializados
  └─ error: falhas controladas e traduzíveis
                         │
                         ▼
SQLite
  ├─ migrações versionadas
  ├─ catálogo híbrido: produtos e ativos
  ├─ saldos independentes por unidade
  ├─ constraints e chaves estrangeiras
  ├─ histórico de movimentações
  └─ auditoria append-only pela aplicação
```

## Decisões principais

1. O frontend nunca abre o arquivo SQLite diretamente. Todo acesso passa por comandos Rust.
2. Escritas relacionadas ocorrem em uma única transação: cadastro, saldo, movimentação e auditoria confirmam ou falham juntos.
3. IDs UUID substituem referências por posição de linha.
4. Datas são armazenadas em ISO 8601 e exibidas no fuso do usuário.
5. O banco usa `foreign_keys`, `WAL`, `busy_timeout` e índices para segurança e resposta previsível.
6. Dados opcionais específicos de categorias ficam em `metadata` JSON; campos importantes para filtro ou integridade devem virar colunas/migrações.
7. O adaptador de demonstração existe apenas para desenvolvimento visual e testes no navegador.
8. `products` guarda o catálogo; `stock_balances` guarda quantidades por unidade; `assets` guarda exemplares patrimoniais; `stock_movements` registra ambos os fluxos.
9. O tipo de controle (`quantity` ou `serialized`) é definido no cadastro do produto e não é inferido pela interface.
10. Ativos possuem um contrato estreito (`AssetGateway`) separado do contrato geral de estoque. A tela depende dessa abstração, não de Tauri ou SQLite.
11. O importador recebe bytes, nunca um caminho arbitrário. `ImportService` analisa o arquivo e mantém a prévia em memória; `Database` recebe somente um lote já validado.
12. Planilhas legadas e tabulares usam adaptadores de leitura diferentes e convergem para o mesmo modelo de candidatos e regras.

## Aplicação de SOLID

- **S — Responsabilidade única:** schemas validam, gateways transportam, hooks orquestram cache, componentes renderizam e o Rust mantém integridade/transações.
- **O — Aberto/fechado:** a interface aceita adaptadores Tauri e demonstração sem alterar a feature; novos adaptadores implementam o mesmo contrato.
- **L — Substituição de Liskov:** os dois gateways respeitam as mesmas entradas, saídas e regras observáveis, permitindo trocar o ambiente sem trocar a tela.
- **I — Segregação de interfaces:** `AssetGateway` expõe somente consulta e atualização de ativos, sem obrigar a feature a depender de cadastro, dashboard ou movimentos.
- **D — Inversão de dependência:** hooks e componentes dependem dos contratos do domínio; detalhes de IPC e banco ficam nas camadas externas.

## Limites dos módulos

- `dashboard`: leitura agregada; não altera estoque.
- `inventory`: catálogo híbrido, saldos agregados e ativos patrimoniais.
- `assets`: consulta detalhada, filtros e atualização transacional de unidade/estado dos exemplares patrimoniais.
- `movements`: entradas, saídas, transferências e ajustes transacionais.
- `master-data`: manutenção de unidades e categorias com exclusão lógica e proteção referencial.
- `audit`: consulta paginada limitada aos 500 eventos recentes, filtros e comparação antes/depois; não oferece mutações.
- `settings`: importação com prévia, criação/listagem de backups e restauração validada com cópia de emergência; preferências permanecem como evolução futura.

## Segurança

- Política de conteúdo restrita no Tauri.
- Capabilities mínimas por janela.
- SQL parametrizado; nomes de tabela dinâmicos são restritos a uma lista interna.
- Erros internos são convertidos em mensagens controladas no IPC.
- Arquivos de banco, log e backup ficam na pasta de dados da aplicação.
- A restauração recebe somente o nome de um arquivo já pertencente à pasta interna; caminhos externos e backups inválidos são recusados.
- Antes de restaurar, o estado corrente é copiado para um backup `pre-restore-*`; a operação concluída também gera auditoria.
- Arquivos de importação têm extensão permitida, limite de 5 MB e no máximo 5.000 linhas. Pré-visualizar não altera o banco.
- A confirmação refaz verificações críticas, cria `pre-import-*` e usa uma única transação; qualquer falha desfaz todo o lote.
