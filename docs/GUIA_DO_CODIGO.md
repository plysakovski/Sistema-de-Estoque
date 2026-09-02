# Guia do código — StockManager Pro Desktop

## 1. Objetivo deste documento

Este guia explica como o projeto está organizado, o papel de cada arquivo, por que cada tecnologia foi escolhida e como uma ação da interface percorre TypeScript, Tauri, Rust e SQLite.

O projeto atual deve ser entendido como uma **fundação profissional funcional**: dashboard, consulta e cadastro de itens já percorrem a arquitetura completa; os demais módulos possuem o lugar correto reservado, mas ainda precisam de regras, comandos e telas próprias.

---

## 2. Modelo mental do sistema

O sistema possui duas partes principais no mesmo aplicativo:

```text
┌────────────────────────────────────────────────────────────┐
│ Interface — React + TypeScript                             │
│ telas, formulários, estados de carregamento e validação    │
└──────────────────────────┬─────────────────────────────────┘
                           │ invoke("nome_do_comando")
                           ▼
┌────────────────────────────────────────────────────────────┐
│ Ponte desktop — Tauri                                     │
│ controla a janela e permite somente comandos autorizados   │
└──────────────────────────┬─────────────────────────────────┘
                           │ função Rust tipada
                           ▼
┌────────────────────────────────────────────────────────────┐
│ Núcleo — Rust                                              │
│ regras, transações, auditoria, backup e tratamento de erro │
└──────────────────────────┬─────────────────────────────────┘
                           │ SQL parametrizado
                           ▼
┌────────────────────────────────────────────────────────────┐
│ Persistência — SQLite                                     │
│ itens, unidades, categorias, movimentos e auditoria        │
└────────────────────────────────────────────────────────────┘
```

A interface não conhece tabelas nem caminhos de arquivos. Ela conhece apenas o contrato `InventoryGateway`. Isso impede que componentes visuais fiquem misturados com SQL ou detalhes do Windows.

---

## 3. Por que esta combinação de tecnologias

### TypeScript

Adiciona verificação de tipos ao JavaScript. Um item não pode mudar silenciosamente de formato entre tela e backend sem que o compilador ou o Zod percebam. Isso reduz erros comuns em campos, filtros e respostas do banco.

### React

Organiza a interface em componentes pequenos e reaproveitáveis. É adequado para dashboard, tabelas, formulários e estados que mudam sem recarregar a janela inteira.

### Vite

Compila o frontend e fornece atualização rápida durante o desenvolvimento. No aplicativo final, gera arquivos estáticos em `dist/`, que o Tauri incorpora ao executável.

### Tauri

Transforma a interface web em aplicativo desktop usando a WebView nativa do Windows. O instalador fica muito menor que uma solução que embarca um navegador completo. O Tauri também cria uma fronteira de segurança entre a interface e o sistema operacional.

### Rust

Concentra operações sensíveis: gravação no banco, transações, backup e auditoria. O compilador impede várias classes de erro de memória e concorrência antes do programa ser distribuído.

### SQLite

É um banco transacional armazenado em um único arquivo e não exige servidor separado. É uma escolha apropriada para um sistema instalado em uma máquina ou utilizado localmente por poucos processos.

### TanStack Query

Gerencia consultas e mutações da interface. Evita chamadas repetidas, representa estados de carregamento/erro e atualiza automaticamente dashboard e lista depois de um cadastro.

### Zod

Valida em tempo de execução aquilo que chega à interface. TypeScript verifica o código durante a compilação; Zod verifica os dados reais recebidos do Rust ou de outra fonte.

### rusqlite

Permite que o Rust acesse SQLite diretamente, com parâmetros e transações. A opção `bundled` incorpora o SQLite ao aplicativo, eliminando a necessidade de instalar banco de dados na máquina do usuário.

---

## 4. Visão geral das pastas

```text
Sistema-de-Estoque/
├─ apps/desktop/           aplicativo instalável
│  ├─ src/                 interface React/TypeScript
│  │  ├─ app/              composição global da aplicação
│  │  ├─ components/       componentes visuais reutilizáveis
│  │  ├─ domain/           contratos e regras independentes de tela
│  │  ├─ features/         módulos funcionais
│  │  ├─ infrastructure/   comunicação com Tauri ou modo demonstração
│  │  ├─ shared/           utilidades compartilhadas
│  │  ├─ styles/           sistema visual e responsividade
│  │  └─ test/             preparação dos testes frontend
│  └─ src-tauri/           núcleo nativo Rust, migrações e permissões
├─ docs/                   documentação central do produto
├─ legacy/                 protótipo inicial preservado
└─ .github/                automação e modelos de colaboração
```

---

## 5. Arquivos da raiz

### `README.md`

É a porta de entrada do projeto. Contém tecnologias, comandos essenciais e pré-requisitos. Deve continuar curto; explicações profundas pertencem a `docs/`.

### `package.json`

É o manifesto do frontend. Define:

- nome e versão;
- dependências usadas em produção;
- ferramentas usadas apenas no desenvolvimento;
- comandos `dev`, `build`, `test`, `lint`, `check` e `tauri`.

O comando `pnpm check` encadeia lint, testes e build. Ele existe para oferecer uma única verificação antes de gerar um instalador.

### `pnpm-lock.yaml`

Registra as versões exatas de todas as dependências TypeScript e suas dependências transitivas. Deve ser versionado no Git para que outra máquina instale o mesmo conjunto testado.

### `pnpm-workspace.yaml`

Contém a configuração de segurança de versões adotada pelo pnpm deste ambiente. As exceções listadas foram registradas quando versões recém-publicadas e verificadas foram instaladas.

### `index.html`

É a página mínima carregada pela WebView. Possui metadados, título e o elemento `<div id="root">`, onde o React monta toda a interface.

### `vite.config.ts`

Configura Vite e Vitest:

- ativa React;
- fixa a porta 5173 esperada pelo Tauri;
- permite host específico para dispositivos Tauri;
- impede que alterações no Rust reiniciem desnecessariamente o Vite;
- usa `jsdom` nos testes da interface.

### `eslint.config.js`

Define as regras de qualidade do TypeScript e dos hooks React. Pastas geradas são ignoradas. A regra de importações de tipos evita carregar código em tempo de execução quando apenas um tipo era necessário.

### `tsconfig.json`

É o agregador das configurações TypeScript. Separa o código da aplicação do código executado pelo Node durante build e testes.

### `tsconfig.app.json`

Configura o TypeScript do frontend com modo estrito, JSX moderno e APIs do navegador. `noUncheckedIndexedAccess` obriga o código a considerar a possibilidade de um índice não existir.

### `tsconfig.node.json`

Configura os arquivos de ferramenta, como `vite.config.ts` e `eslint.config.js`, que executam no Node e não dentro da WebView.

### `rust-toolchain.toml`

Pede o canal estável do Rust e inclui `rustfmt` e `clippy`. Isso padroniza formatação e análise estática em outras máquinas.

### `.gitignore`

Impede que dependências, builds, bancos locais, arquivos WAL, logs e configurações privadas sejam enviados ao Git.

### `app-icon.svg`

É a fonte vetorial do ícone do StockManager Pro. Todos os PNGs, ICO e ICNS podem ser regenerados a partir dele com `pnpm.cmd tauri icon app-icon.svg`.

---

## 6. Entrada e composição do frontend

### `src/main.tsx`

É o primeiro arquivo TypeScript executado. Ele:

1. encontra o elemento `root` do HTML;
2. ativa o `StrictMode` do React;
3. disponibiliza o TanStack Query para todos os componentes;
4. monta o `AppShell`;
5. importa o CSS global.

Ele deve permanecer pequeno. Regras de negócio não pertencem aqui.

### `src/app/app-shell.tsx`

É a estrutura permanente da aplicação:

- barra lateral;
- navegação;
- barra superior;
- região principal;
- barra de status;
- abertura do diálogo de cadastro.

O tipo `PageId` limita as páginas possíveis. Atualmente a troca de páginas usa estado local, suficiente para um desktop de janela única. Se futuramente houver URLs internas, histórico ou links profundos, este ponto pode migrar para um roteador.

O menu inicia recolhido em telas pequenas e aberto em desktop. A navegação móvel fecha o menu depois da escolha.

### `src/app/query-client.ts`

Configura o cache de consultas:

- dados permanecem atuais por 30 segundos;
- uma falha recebe uma nova tentativa;
- voltar o foco para a janela não recarrega tudo automaticamente.

Essas decisões evitam consultas excessivas em um banco local, mantendo a interface responsiva.

---

## 7. Domínio e contratos

### `src/domain/inventory.ts`

É o contrato central do frontend. Define:

- formatos de produto, ativo, unidade, movimento e dashboard;
- modelos de controle `quantity` e `serialized`;
- estados aceitos para ativos;
- regras condicionais dos formulários de cadastro e movimentação;
- interface `InventoryGateway`.

Os schemas Zod geram os tipos TypeScript e também validam dados reais. Esta pasta não importa React, Tauri ou CSS, portanto suas regras podem ser testadas isoladamente.

### `src/domain/inventory.test.ts`

Verifica três regras iniciais:

- o dashboard de demonstração respeita o contrato;
- estoque negativo é recusado.
- ativo serializado com saldo inicial exige patrimônio.

À medida que o sistema crescer, novos casos de negócio devem ser adicionados aqui e em testes Rust.

### `src/domain/master-data.ts`

Define o contrato compartilhado de Unidades e Categorias: identificador, nome, código, estado ativo, quantidade de vínculos e atualização. O schema também limita códigos a letras, números, hífen e sublinhado.

### `src/domain/master-data.test.ts`

Testa códigos válidos e recusa espaços ou símbolos que tornariam os identificadores inconsistentes.

### `src/domain/assets.ts`

Define somente o contexto de ativos detalhados: registro enriquecido com produto/categoria, filtros, estados editáveis, entrada de atualização e o contrato estreito `AssetGateway`. O estado `disposed` pode ser consultado, mas não integra o schema de edição direta.

Essa separação aplica segregação de interface e inversão de dependência: a feature não conhece comandos Tauri nem consultas SQLite.

### `src/domain/assets.test.ts`

Testa o resumo por estado e garante que a edição comum não aceite reativar ou baixar diretamente um equipamento.

### `src/domain/audit.ts`

Define o registro de auditoria, filtros, contrato `AuditGateway` e a transformação pura que compara os objetos `beforeData` e `afterData`. O contrato aceita novos nomes de ação e entidade sem exigir alteração estrutural, mantendo o módulo aberto para evolução.

### `src/domain/audit.test.ts`

Verifica a comparação legível dos campos e recusa intervalos de datas invertidos.

### `src/domain/settings.ts`

Define os contratos de backup, resultado de restauração e `SettingsGateway`. A interface conhece somente nomes internos, datas, tamanhos e estados de integridade; caminhos absolutos permanecem encapsulados no Rust.

### `src/domain/settings.test.ts`

Valida registros de backup e impede solicitações de restauração sem um destino identificado.

### `src/domain/import.ts` e `src/domain/import.test.ts`

Definem os contratos da prévia, das linhas, do resultado e do `ImportGateway`. A função pura `canConfirmImport` concentra a regra que libera a confirmação somente quando existe token e nenhum erro.

---

## 8. Infraestrutura e modo demonstração

### `src/infrastructure/inventory-gateway.ts`

Implementa o padrão Gateway. Existem duas implementações do mesmo contrato:

- `TauriInventoryGateway`: chama comandos Rust com `invoke`;
- `MockInventoryGateway`: trabalha em memória para abrir a interface no navegador.

`isTauriRuntime()` seleciona automaticamente a implementação. Essa escolha permite desenvolver e testar o visual com `pnpm dev`, mas usar SQLite real dentro do executável instalado.

As respostas Tauri passam pelo Zod antes de chegar às telas. Assim, uma mudança incompatível no Rust é detectada na fronteira.

### `src/infrastructure/master-data-gateway.ts`

Seleciona os comandos corretos para Unidade ou Categoria e mantém a tela genérica desacoplada dos nomes IPC. No navegador, implementa as mesmas regras em memória, incluindo duplicidade e bloqueio de desativação com vínculos.

### `src/infrastructure/assets-gateway.ts`

Implementa `AssetGateway` com dois adaptadores substituíveis. O adaptador Tauri valida cada resposta Rust com Zod; o adaptador de demonstração monta registros enriquecidos a partir do seed e reproduz filtros e proteções para testes visuais no navegador.

### `src/infrastructure/audit-gateway.ts`

Mantém a tela independente da origem dos eventos. No aplicativo instalado chama `list_audit_logs`; no navegador oferece registros demonstrativos e os mesmos filtros observáveis.

### `src/infrastructure/settings-gateway.ts`

Implementa criação, listagem e restauração para Tauri e demonstração. O mock permite testar o fluxo visual sem tocar em arquivos reais; o adaptador instalado valida todas as respostas com Zod.

### `src/infrastructure/import-gateway.ts`

Envia nome, bytes e unidade ao Rust sem conceder acesso arbitrário ao sistema de arquivos. O adaptador de demonstração produz uma prévia realista para testes visuais; a aplicação instalada usa o parser Rust.

### `src/infrastructure/mock/master-data.ts`

É o estado compartilhado dos cadastros no modo navegador. Dashboard, formulários e telas administrativas leem a mesma coleção, portanto a demonstração permanece sincronizada durante a sessão.

### `src/infrastructure/mock/seed.ts`

Contém produtos, ativos, unidades e movimentos usados somente pelo adaptador de navegador. Ele também constrói o dashboard demonstrativo. O mock permite cadastrar produtos e executar movimentações em memória para validar a interface.

Esses dados não são o banco SQLite do aplicativo instalado. Alterações feitas no navegador desaparecem ao recarregar a página.

---

## 9. Feature Dashboard

### `src/features/dashboard/use-dashboard.ts`

É o hook que liga a tela ao gateway. A chave `dashboard` identifica o dado no cache do TanStack Query.

Separar o hook da tela facilita testar, substituir a fonte de dados e reutilizar a consulta.

### `src/features/dashboard/dashboard-page.tsx`

Renderiza:

- cabeçalho e horário de atualização;
- quatro indicadores;
- distribuição por unidade;
- itens abaixo do mínimo;
- movimentações recentes.

O componente trata explicitamente carregamento e erro antes de usar os dados. O tamanho das barras é calculado proporcionalmente ao maior estoque.

O painel de equipamentos em manutenção consulta os ativos reais e deixa de usar um número fixo da demonstração.

---

## 10. Feature Estoque

### `src/features/inventory/use-inventory.ts`

Possui três hooks:

- `useInventory`: consulta a lista usando o texto pesquisado como parte da chave de cache;
- `useLocations`: fornece as unidades válidas aos formulários;
- `useCreateItem`: executa o cadastro e invalida, em paralelo, os caches de estoque e dashboard.

Invalidar significa marcar os dados como desatualizados para que a interface busque novamente o estado verdadeiro.

### `src/features/inventory/inventory-page.tsx`

Mostra o catálogo híbrido, identifica visualmente o modelo por quantidade ou patrimonial e resume a distribuição entre unidades. A pesquisa também encontra SKU, patrimônio e número de série. `useDeferredValue` evita que uma tabela grande seja recalculada com prioridade máxima a cada tecla digitada.

A tabela usa rolagem horizontal quando necessário, preservando colunas em telas menores.

### `src/features/inventory/create-item-dialog.tsx`

Controla o formulário de cadastro adaptativo:

1. mantém os valores digitados em estado React;
2. permite escolher controle por quantidade ou ativo individual;
3. mostra patrimônio e série somente para o modelo serializado;
4. converte quantidades para número;
5. valida com `createItemInputSchema`;
6. chama `useCreateItem`;
7. limpa e fecha somente depois do sucesso.

O diálogo usa atributos de acessibilidade como `role="dialog"`, `aria-modal` e associação com o título.

### `src/features/assets/use-assets.ts`

Orquestra consulta filtrada e atualização. Depois de salvar, invalida em paralelo os caches de ativos, estoque, movimentações e dashboard, garantindo que cada tela volte a ler o estado verdadeiro.

### `src/features/assets/assets-page.tsx`

Exibe indicadores do conjunto filtrado, busca adiada, filtros de unidade/estado e tabela detalhada. Ativos baixados permanecem visíveis, porém sua edição fica desabilitada.

### `src/features/assets/edit-asset-dialog.tsx`

Possui uma única responsabilidade: coletar nova unidade, estado operacional e observação. A validação fica no domínio e a gravação no hook/gateway. Mudanças de unidade e estado são confirmadas juntas pelo Rust.

### `src/features/audit/use-audit.ts`

Consulta o gateway usando os filtros como parte da chave do TanStack Query. Como a feature é somente leitura, não expõe mutações nem invalidações.

### `src/features/audit/audit-page.tsx`

Exibe resumo, busca adiada, filtros por entidade/ação/período e os 500 eventos mais recentes. Cada linha resolve um nome amigável, mas preserva o identificador imutável para rastreabilidade.

### `src/features/audit/audit-details-dialog.tsx`

Apresenta metadados e comparação campo a campo entre o estado anterior e posterior, sem expor comandos de alteração ou exclusão.

### `src/features/settings/use-settings.ts`

Separa consultas e mutações. Criar backup atualiza somente a lista; restaurar invalida todos os caches porque estoque, cadastros, ativos, dashboard e auditoria podem retornar a um estado anterior.

### `src/features/settings/settings-page.tsx`

Mostra as proteções do armazenamento, cria backups e lista arquivo, tipo, data, tamanho e integridade. Backups inválidos permanecem visíveis para diagnóstico, mas não podem ser restaurados.

### `src/features/settings/restore-backup-dialog.tsx`

Exige a confirmação textual `RESTAURAR`, explica a substituição do estado atual e informa que uma cópia de emergência será criada antes da operação.

### `src/features/settings/use-import.ts` e `import-spreadsheet-dialog.tsx`

Orquestram análise, descarte e confirmação. O diálogo exige unidade, lê no máximo 5 MB, mostra totais e erros por linha e mantém o botão final bloqueado até a prévia estar integralmente válida.

### `src/features/movements/use-movements.ts`

Agrupa as consultas de movimentos e ativos de um produto, além da mutação que registra uma operação. Depois do sucesso, invalida em paralelo movimento, estoque, ativos selecionados e dashboard.

### `src/features/movements/movements-page.tsx`

Substitui o antigo espaço reservado por uma tabela operacional com até 200 movimentos. Patrimônios aparecem abaixo do nome quando a operação envolve um ativo individual.

### `src/features/movements/create-movement-dialog.tsx`

Monta o formulário conforme produto e operação. Produtos por quantidade pedem quantidade e unidades; ativos pedem patrimônio na entrada ou seleção do exemplar nas demais operações. Transferência, baixa e mudança de estado usam o mesmo comando transacional.

### `src/features/master-data/use-master-data.ts`

Oferece consulta, gravação e ativação/desativação para os dois tipos de cadastro. As invalidações atualizam em paralelo tabela, dashboard e opções de unidade usadas nos formulários.

### `src/features/master-data/master-data-page.tsx`

É uma tela parametrizada, usada tanto por Unidades quanto por Categorias. Mostra vínculos, estado, atualização e ações. A reutilização garante o mesmo comportamento visual sem duplicar duas páginas quase idênticas.

### `src/features/master-data/master-data-dialog.tsx`

Atende criação e edição. Sugere automaticamente um código normalizado a partir do nome, mas permite ajuste manual. Erros do Rust são apresentados sem fechar o diálogo.

### `src/features/placeholder/placeholder-page.tsx`

É uma tela temporária ainda reutilizada por Auditoria e Configurações.

Seu objetivo é manter a navegação e a divisão de módulos prontas sem fingir que fluxos ainda não implementados estão funcionais. Cada placeholder deverá ser substituído pela feature correspondente.

---

## 11. Componentes e utilidades

### `src/components/metric.tsx`

Componente reaproveitável dos indicadores do dashboard. Recebe rótulo, valor, ícone e tom visual. Evita repetir marcação e classes em quatro lugares.

### `src/components/empty-state.tsx`

Padroniza mensagens de lista vazia. É usado quando não há itens abaixo do estoque mínimo.

### `src/shared/format.ts`

Centraliza a formatação brasileira de data/hora e a tradução dos tipos de movimento. Evita que cada tela implemente traduções diferentes.

### `src/styles/global.css`

Contém o sistema visual completo:

- tokens de cor, borda e raio em `:root`;
- shell, menu e barras da aplicação;
- indicadores, gráfico, painéis e tabelas;
- diálogo e campos de formulário;
- breakpoints em 1050 px e 720 px;
- suporte a `prefers-reduced-motion`;
- `content-visibility` para reduzir trabalho de renderização em tabelas longas.

Neste estágio um único CSS facilita enxergar o sistema visual completo. Quando novas features crescerem, estilos específicos podem ser movidos para perto de cada módulo, preservando os tokens globais.

### `src/lucide-modules.d.ts`

Declara para o TypeScript o formato dos imports individuais de ícones Lucide. Os imports diretos evitam processar o pacote inteiro durante o desenvolvimento.

### `src/vite-env.d.ts`

Inclui os tipos fornecidos pelo Vite, como variáveis de ambiente e imports de recursos.

### `src/test/setup.ts`

Limpa o DOM depois de cada teste React para evitar que um teste contamine o seguinte.

---

## 12. Configuração do aplicativo Tauri

### `src-tauri/Cargo.toml`

É o manifesto Rust. Define o pacote, versão mínima de Rust, formatos de biblioteca e dependências:

- `tauri`: aplicação desktop;
- `rusqlite`: SQLite;
- `parking_lot`: bloqueio eficiente da conexão compartilhada;
- `serde` e `serde_json`: conversão entre Rust e JSON;
- `uuid`: identificadores estáveis;
- `chrono`: datas;
- `thiserror`: erros tipados;
- plugins de log e instância única.

### `src-tauri/Cargo.lock`

Trava as versões exatas de todas as crates Rust. Deve ser versionado porque este é um aplicativo final, não uma biblioteca pública.

### `src-tauri/build.rs`

Executa `tauri_build::build()` durante a compilação. Ele gera recursos do Windows, processa a configuração e incorpora metadados e ícones.

### `src-tauri/tauri.conf.json`

Configura:

- nome, versão e identificador do aplicativo;
- comandos usados antes de `dev` e `build`;
- localização do frontend compilado;
- tamanho inicial e mínimo da janela;
- política de segurança de conteúdo;
- idiomas e formatos dos instaladores;
- ícones usados pelo executável e bundles.

O `frontendDist` aponta para `../dist`, pois o Tauri empacota o resultado do Vite, não o código-fonte React.

### `src-tauri/capabilities/default.json`

Declara as permissões da janela principal. O princípio é conceder apenas o necessário: funções básicas do Tauri e log. Novos plugins devem adicionar permissões conscientemente aqui.

---

## 13. Entrada e inicialização do Rust

### `src-tauri/src/main.rs`

É o executável mínimo. Apenas chama `stockmanager_lib::run()`. Separar o executor da biblioteca é o padrão Tauri que também permite outros alvos e testes.

### `src-tauri/src/lib.rs`

Monta a aplicação nativa:

1. registra o plugin de instância única;
2. registra logs com limite de tamanho;
3. encontra a pasta de dados do aplicativo;
4. abre ou cria `stockmanager.db`;
5. disponibiliza `Database` como estado compartilhado;
6. registra os comandos permitidos para o frontend;
7. inicia o loop da janela.

A instância única impede duas cópias independentes do aplicativo de disputarem o mesmo banco.

---

## 14. Fronteira Tauri e modelos Rust

### `src-tauri/src/commands.rs`

Contém os comandos acessíveis pela interface:

- `get_dashboard`;
- `list_items`;
- `create_item`;
- `list_locations`;
- `list_assets`;
- `list_movements`;
- `create_movement`;
- `list_location_records`, `save_location` e `set_location_active`;
- `list_categories`, `save_category` e `set_category_active`;
- `create_backup`.
- `preview_import`, `confirm_import` e `discard_import`.

Essas funções são finas de propósito. Elas recebem parâmetros, chamam `Database` e devolvem resultado. Regras complexas devem ficar no domínio/serviço ou na camada de banco, não na ponte IPC.

`create_backup` grava uma cópia com data e hora dentro da pasta de dados do aplicativo. Os comandos de importação mantêm a fronteira fina: contexto vem do banco, análise fica em `ImportService` e confirmação volta ao banco.

### `src-tauri/src/imports.rs`

É o serviço de importação. Usa Calamine para ler formatos Excel/ODS, um parser CSV UTF-8, adaptadores para formato tabular e legado, normalização comum, detecção de duplicidades e prévias temporárias em memória. Nenhuma função desse módulo grava SQLite.

### `src-tauri/src/models.rs`

Define os objetos enviados e recebidos pela fronteira Tauri. Rust usa `snake_case`; `#[serde(rename_all = "camelCase")]` converte automaticamente para o padrão TypeScript, por exemplo `asset_tag` → `assetTag`.

Manter modelos explícitos evita devolver linhas do banco sem controle.

### `src-tauri/src/error.rs`

Define erros compreensíveis do aplicativo:

- banco;
- entrada/saída;
- validação;
- registro ausente;
- pasta de dados indisponível.

O erro é serializado como texto para atravessar o Tauri. No futuro, pode evoluir para códigos estruturados e traduções específicas na interface.

---

## 15. Banco e regras transacionais

### `src-tauri/src/database.rs`

É atualmente o núcleo funcional do backend.

#### Abertura

`Database::open` cria a pasta, abre o arquivo, configura SQLite, aplica as migrações `0001` e `0002` e insere dados iniciais somente quando o catálogo está vazio.

Configurações importantes:

- `foreign_keys = ON`: aplica relacionamentos;
- `journal_mode = WAL`: melhora segurança e concorrência entre leitura e escrita;
- `synchronous = NORMAL`: equilíbrio entre desempenho e durabilidade;
- `busy_timeout = 5000`: espera brevemente quando o banco está ocupado.

#### Conexão compartilhada

A conexão fica em `Mutex<Connection>`. Somente uma operação a utiliza de cada vez, o que simplifica a segurança nesta fase local. Se o volume crescer, pode ser substituída por um pool ou worker dedicado sem mudar o contrato do frontend.

#### Dashboard

Executa consultas agregadas para total de itens, unidades, categorias, alertas, ativos em manutenção, estoque por unidade e movimentos recentes. O total combina saldos quantitativos com a contagem de ativos não baixados.

#### Cadastro

`create_item` valida a entrada e abre uma transação. Dentro dela:

1. localiza ou cria categoria;
2. cria o produto e fixa seu modelo de controle;
3. cria saldo por unidade ou ativo patrimonial inicial;
4. registra a entrada inicial;
5. registra auditoria;
6. confirma tudo de uma vez.

Se qualquer etapa falhar, nenhuma das anteriores fica parcialmente salva.

Depois do `commit`, a conexão é liberada antes de consultar novamente o item. Essa liberação explícita evita um deadlock.

#### Busca

Usa parâmetros SQL para pesquisar SKU, nome, categoria, patrimônio e número de série. Os valores do usuário não são concatenados diretamente ao SQL.

#### Movimentações

`create_movement` consulta o modelo do produto dentro da transação. Para quantidade, altera `stock_balances` e impede saldo negativo. Para ativos, cria ou atualiza um exemplar específico. Transferências debitam e creditam as unidades na mesma transação; ajustes quantitativos recebem o novo saldo físico; ajustes patrimoniais mudam o estado do equipamento.

#### Unidades e Categorias

Os mesmos helpers internos mantêm ambos os cadastros, mas a escolha da tabela é feita por um enum Rust fechado, nunca por texto vindo da interface. Alterações geram auditoria. A desativação é recusada quando uma unidade possui saldo/ativos ou quando uma categoria possui produtos. Como é exclusão lógica, um registro livre pode ser reativado sem perder seu histórico.

#### Backup

Usa `VACUUM INTO` para produzir um arquivo SQLite consistente no destino.

#### Importação

`import_context` fornece somente os identificadores necessários à prévia. `import_stage` cria uma cópia `pre-import-*`, revalida unidade/SKUs e grava produto, categoria, saldo ou ativo, movimento e auditoria em uma única transação. Restrições SQLite continuam sendo a última linha de defesa e provocam rollback integral.

#### Dados iniciais

`seed_database` insere usuário, unidades, categorias, produtos, saldos e ativos de demonstração quando não existe catálogo. Instalações provenientes da primeira demo são detectadas pela migração e não recebem um segundo conjunto de produtos.

**Antes de uso real**, esta carga deve ser colocada atrás de uma opção “Criar dados de demonstração” ou removida. Hoje ela é intencional para que a primeira execução não abra vazia.

#### Teste Rust

O módulo `tests` abre SQLite em memória e cobre criação com saldo inicial, bloqueio de saída acima do saldo, transferência patrimonial sem alterar o total e migração de um registro legado sem perda de quantidade.

### `src-tauri/migrations/0001_initial.sql`

Cria o primeiro esquema do banco:

- `schema_migrations`: versões aplicadas;
- `users`: usuários e papéis;
- `locations`: unidades;
- `categories`: categorias;
- `items`: saldo, mínimo, status e metadados;
- `movements`: histórico operacional;
- `audit_logs`: trilha de alterações;
- `app_settings`: configurações.

Também cria índices para pesquisas e históricos. Restrições `CHECK`, `UNIQUE` e chaves estrangeiras protegem o banco mesmo que uma futura tela tenha um defeito.

Migrações publicadas nunca devem ser editadas.

### `src-tauri/migrations/0002_hybrid_inventory.sql`

Adiciona o modelo híbrido:

- `products`: catálogo e tipo de controle;
- `stock_balances`: saldo de um produto por unidade;
- `assets`: exemplares individualizados por patrimônio e série;
- `stock_movements`: histórico comum aos dois modelos.

Também importa os antigos registros de `items` como produtos por quantidade. As tabelas legadas permanecem no banco para que a atualização seja conservadora e recuperável por backup; o código novo passa a ler e escrever apenas nas tabelas híbridas.

---

## 16. Ícones

### `src-tauri/icons/icon.ico`

Ícone em múltiplos tamanhos exigido pelo executável e instalador Windows.

### `src-tauri/icons/icon.icns`

Formato de ícone usado pelo macOS.

### `32x32.png`, `64x64.png`, `128x128.png`, `128x128@2x.png` e `icon.png`

Versões PNG usadas por desktop e empacotadores.

### `Square*Logo.png` e `StoreLogo.png`

Variações preparadas para pacotes Windows/AppX e Microsoft Store.

### `icons/android/**`

Ícones adaptativos em densidades diferentes para um eventual build Android.

### `icons/ios/**`

Conjunto de tamanhos exigido por iPhone e iPad.

Todos esses arquivos são gerados automaticamente a partir de `app-icon.svg`; não devem ser editados individualmente.

---

## 17. Documentação existente

### `docs/ARCHITECTURE.md`

Resumo das camadas, limites entre módulos e decisões de segurança.

### `docs/DATA_MODEL.md`

Resumo das tabelas e invariantes do banco, incluindo a possível evolução para ativos serializados e saldos por unidade.

### `docs/DEVELOPMENT.md`

Convenções para novas features, migrações, importação das planilhas e geração de instaladores.

### `docs/DESIGN_SYSTEM.md`

Registra cores, tipografia, densidade e princípios da interface.

### `docs/design/dashboard-concept.png`

Conceito visual usado como referência na implementação do dashboard.

### `docs/GUIA_DO_CODIGO.md`

Este documento. Deve ser atualizado quando a arquitetura ou os fluxos principais mudarem.

---

## 18. Pastas geradas automaticamente

### `node_modules/`

Dependências TypeScript instaladas por `pnpm install`. Não editar e não versionar.

### `dist/`

Resultado de `pnpm build`. Contém HTML, CSS e JavaScript otimizados que entram no aplicativo. Não editar manualmente.

### `src-tauri/target/`

Resultado das compilações Rust. Inclui caches, executável e instaladores.

Arquivos finais importantes:

```text
src-tauri/target/release/stockmanager-desktop.exe
src-tauri/target/release/bundle/nsis/StockManager Pro_1.0.0_x64-setup.exe
src-tauri/target/release/bundle/msi/StockManager Pro_1.0.0_x64_pt-BR.msi
```

---

## 19. Fluxos completos

### 19.1 Inicialização do aplicativo instalado

```text
main.rs
  → lib.rs::run
  → encontra pasta de dados
  → Database::open
  → configura SQLite
  → aplica migração
  → registra comandos Tauri
  → carrega dist/index.html
  → src/main.tsx
  → AppShell
  → DashboardPage
  → invoke("get_dashboard")
  → commands::get_dashboard
  → Database::dashboard
  → SQLite
```

### 19.2 Cadastro de item

```text
botão Novo item
  → CreateItemDialog
  → validação Zod
  → useCreateItem
  → InventoryGateway.createItem
  → invoke("create_item")
  → commands::create_item
  → Database::create_item
  → transação SQLite
       ├─ categoria/unidade
       ├─ item
       ├─ movimento inicial
       └─ auditoria
  → resposta validada pelo Zod
  → invalida caches de estoque e dashboard
  → telas atualizadas
```

### 19.3 Execução no navegador

```text
pnpm dev
  → não existe __TAURI_INTERNALS__
  → MockInventoryGateway
  → seed.ts em memória
```

### 19.4 Execução instalada

```text
StockManager Pro.exe
  → existe __TAURI_INTERNALS__
  → TauriInventoryGateway
  → comandos Rust
  → stockmanager.db
```

---

## 20. O que está pronto e o que ainda é demo

### Implementado de ponta a ponta

- inicialização do banco;
- migração inicial e migração híbrida compatível;
- dashboard real no aplicativo instalado;
- consulta e busca por produto, SKU, patrimônio e série;
- cadastro transacional de produto quantitativo ou patrimonial;
- criação automática de categoria durante cadastro;
- movimento de entrada inicial;
- entradas, saídas, transferências e ajustes;
- saldo independente por unidade;
- estados de ativo: disponível, em uso, manutenção e baixado;
- tela detalhada de ativos com busca, filtros e indicadores;
- alteração transacional de unidade e estado do ativo;
- proteção de ativo baixado como registro somente leitura;
- bloqueio de saldo negativo;
- tela completa de movimentações;
- criação, edição, ativação e desativação de unidades;
- criação, edição, ativação e desativação de categorias;
- proteção contra desativação de cadastros vinculados;
- auditoria do cadastro;
- auditoria de todas as movimentações;
- tela de auditoria somente leitura com busca, filtros e detalhes antes/depois;
- comando de backup;
- tela de backup e recuperação;
- validação de integridade e estrutura dos backups;
- restauração restrita à pasta interna com cópia pré-restauração;
- auditoria da restauração concluída;
- importação XLSX/XLS/XLSB/ODS/CSV com formatos tabular e legado;
- prévia sem gravação, erros por linha e descarte de sessões;
- backup pré-importação, auditoria e rollback integral;
- autenticação local, sessões e bloqueio após tentativas inválidas;
- perfis operador, gestor e administrador;
- escopo de autorização por unidade validado no backend;
- solicitações de ajuste e reposição com decisão segregada;
- movimentações em lote com até 200 linhas;
- lotes de recebimento e ativos individualizados por patrimônio/série;
- ocorrências de ativos e fluxo de manutenção;
- recebimento vinculado aos produtos aprovados na reposição;
- entregas parciais com atualização do progresso da solicitação;
- modo navegador para desenvolvimento;
- instalador NSIS em formato EXE.

### Pendente ou reservado

- Busca global: campo visual ainda sem comportamento.
- Transferência aprovada: despacho e recebimento em duas etapas ainda não implementados.
- Distribuição institucional: assinatura Authenticode e atualização assinada ainda pendentes.
- Produção: exige homologação, gestão operacional de chaves, teste de restauração e avaliação de segurança independente.

### Modelo híbrido adotado

A limitação da primeira demo foi removida. O banco agora separa:

```text
products        catálogo/SKU e tipo de controle
assets          equipamentos serializados
stock_balances  saldo de um produto por unidade
stock_movements histórico dos dois modelos
```

Os antigos `items` são migrados como produtos por quantidade para preservar dados já instalados.

---

## 21. Onde alterar cada tipo de requisito

| Necessidade | Arquivo inicial |
|---|---|
| Mudar cores ou espaçamentos | `src/styles/global.css` |
| Mudar menu ou navegação | `src/app/app-shell.tsx` |
| Alterar indicadores | `src/features/dashboard/dashboard-page.tsx` e `database.rs` |
| Adicionar campo ao item | migração nova, `models.rs`, `database.rs`, `inventory.ts` e formulário |
| Criar novo comando | `commands.rs`, registro em `lib.rs` e gateway TypeScript |
| Criar nova tabela | nova migração SQL |
| Criar nova tela | nova pasta em `src/features/` e entrada no shell |
| Alterar cache/recarga | hooks da feature e `query-client.ts` |
| Adicionar permissão/plugin | `Cargo.toml`, `lib.rs` e `capabilities/default.json` |
| Alterar instalador/janela | `tauri.conf.json` |
| Alterar ícone | `app-icon.svg` e comando `tauri icon` |

---

## 22. Ordem recomendada para continuar

1. Implementar despacho e recebimento em duas etapas para transferências aprovadas.
2. Concluir assinatura Authenticode, atualização assinada e procedimentos de recuperação.
3. Implementar a busca global entre produtos, ativos, lotes e solicitações.
4. Ampliar testes de interface e homologar todos os perfis antes da distribuição real.

---

## 23. Comandos do dia a dia

```powershell
# Interface no navegador, usando dados em memória
pnpm.cmd dev

# Aplicativo Tauri em desenvolvimento, usando SQLite real
pnpm.cmd tauri dev

# Lint + testes frontend + build frontend
pnpm.cmd check

# Testes Rust
cargo test --manifest-path src-tauri/Cargo.toml

# Análise estática Rust
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings

# Gerar instaladores
pnpm.cmd tauri build
```

---

## 24. Glossário rápido

- **Componente:** parte reutilizável da interface React.
- **Hook:** função React que encapsula estado ou acesso a dados.
- **Gateway:** contrato que esconde a origem real dos dados.
- **IPC:** comunicação entre a WebView e o núcleo Rust.
- **Schema:** descrição e validação da forma de um dado.
- **Migração:** mudança versionada na estrutura do banco.
- **Transação:** grupo de gravações que confirma tudo ou desfaz tudo.
- **WAL:** modo do SQLite que melhora segurança e convivência de leituras/escritas.
- **UUID:** identificador global que não depende da posição de uma linha.
- **Cache:** cópia temporária usada para evitar consultas repetidas.
- **Invalidação:** marcação de um cache como desatualizado.
- **Bundle:** pacote final distribuível, como EXE ou MSI.

---

## Conclusão

A principal escolha estrutural foi manter a interface, os contratos e a persistência separados. Isso aumenta a quantidade inicial de arquivos, mas reduz o custo de cada mudança futura: uma nova tela não precisa conhecer SQLite, e uma alteração no banco não precisa espalhar SQL pelos componentes React.

O projeto já prova o caminho completo entre formulário, validação, comando nativo, transação e atualização da interface. A continuação deve preservar esse mesmo fluxo para cada novo caso de uso.
