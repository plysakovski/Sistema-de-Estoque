# Regras de negócio do StockManager Pro

## Objetivo do produto

O StockManager Pro controla estoque por quantidade e ativos patrimoniais serializados em uma ou mais unidades. O sistema mantém rastreabilidade das decisões e separa operações rotineiras, correções excepcionais e abastecimento.

## Perfis e responsabilidades

### Operador

- Acessa somente as unidades às quais foi vinculado.
- Consulta catálogo, saldos, ativos e movimentações dentro desse escopo.
- Registra saídas rotineiras da própria unidade.
- Pode registrar uma saída com vários produtos em um único lote operacional.
- Relata defeitos de ativos da própria unidade; o ativo fica indisponível até análise.
- Solicita ajustes quando a contagem física diverge do sistema.
- Solicita reposição de produtos existentes no catálogo.
- Pode alternar ativos da própria unidade entre `available` (Disponível) e `in_use` (Em uso), pois essas são transições operacionais rotineiras.
- Não transfere ativos, registra manutenção, baixa ou corrige dados diretamente.

### Gestor

- Acessa somente as unidades sob sua responsabilidade.
- Possui as capacidades do operador dentro desse escopo.
- Mantém catálogo, categorias e políticas de estoque por unidade.
- Registra entradas, saídas, transferências e ajustes em lote dentro do próprio escopo.
- Analisa ajustes e reposições de outro usuário.
- Decide se uma reposição será atendida por transferência, compra, combinação das duas ou recusa.
- Consulta a auditoria operacional do seu escopo e executa rotinas de backup/importação.

### Administrador

- Possui visão global para administração técnica.
- Gerencia usuários, vínculos com unidades e cadastro de unidades.
- Gerencia criptografia, recuperação e restauração de backup.
- Pode executar operações emergenciais globais, sempre registradas em auditoria.

## Segregação dos fluxos

### Ajuste

Corrige um dado que deveria representar a realidade atual. Exige descrição, solicitante, revisor diferente do solicitante, parecer e auditoria. A aprovação aplica a correção de forma transacional.

### Reposição

Atende uma necessidade futura de uma unidade. Um pedido contém uma justificativa, prioridade e um ou mais produtos do catálogo. O saldo observado na abertura é preservado como fotografia histórica.

O gestor pode dividir cada item entre transferência e compra. A quantidade aprovada nunca pode exceder a solicitada. Transferências reservam o saldo ou os patrimônios específicos na origem durante a decisão, impedindo dupla promessa.

Uma transferência aprovada ocorre em duas etapas. Um gestor da origem despacha itens reservados e o sistema os marca como `em trânsito`, sem creditar o destino. Um operador, gestor ou administrador vinculado ao destino confere o despacho e informa quantidades recebidas e recusadas. A recusa exige justificativa, devolve o item à reserva da origem e permite novo despacho. Recebimentos parciais preservam o saldo restante em trânsito.

### Movimentação

Registra um fato físico que já ocorreu. Não substitui ajuste nem reposição. A saída rotineira é permitida ao operador; entradas, transferências e alterações excepcionais exigem nível gestor ou fluxo previamente aprovado.

Um lote agrupa até 200 movimentações do mesmo tipo em uma única transação. Cada linha permanece individual no histórico e compartilha o identificador do lote, documento, responsável e data. Se qualquer linha falhar, nenhuma alteração do lote é persistida.

## Catálogo, ativos e recebimento

- Cadastrar produto cria somente o tipo de produto no catálogo e nunca altera saldo.
- Produtos por quantidade não usam patrimônio ou série.
- Produtos patrimoniais geram um ativo individual para cada unidade recebida.
- Patrimônio é sempre obrigatório e único; a série segue a política configurada no produto.
- Quando a política for `required`, o lote não é confirmado enquanto todas as séries não forem informadas.
- Todo ativo recebido fica vinculado ao lote que o originou.
- Reposições aprovadas para compra oferecem recebimento em lote vinculado à solicitação e limitado à quantidade aprovada.

## Ocorrência de ativo

O operador informa o ativo, a pessoa que o devolveu e a descrição do problema. O sistema cria uma ocorrência auditável e move imediatamente o ativo para manutenção, impedindo nova entrega. O gestor decide entre reparo interno, assistência externa, garantia, resolução ou baixa. A resolução devolve o ativo ao estado disponível; a baixa encerra sua vida operacional.

## Estados da reposição

- `pending`: aguarda análise.
- `approved`: todos os itens foram integralmente aprovados.
- `partially_approved`: apenas parte da quantidade foi aprovada.
- `rejected`: nenhuma quantidade foi aprovada.
- `in_fulfillment`: compra ou transferência está em execução.
- `fulfilled`: todos os itens aprovados foram recebidos.
- `cancelled`: cancelamento administrativo com justificativa.

Na versão 1.8.0, compras aprovadas podem ser recebidas em lote e os ativos serializados são individualizados por patrimônio e série.

Na versão 1.8.1, o recebimento vinculado a uma reposição deixa de ser uma movimentação livre. A tela é preenchida somente com os produtos aprovados ainda pendentes, bloqueia a troca ou inclusão de produtos externos e exige referência da compra. Produtos serializados geram uma linha por unidade; entregas parciais são registradas desmarcando explicitamente as unidades ainda não recebidas.

Na versão 1.9.0, transferências aprovadas ganham despacho e recebimento independentes. O saldo quantitativo sai da origem no despacho e chega ao destino somente na conferência. Ativos patrimoniais são reservados e transportados por identificador individual. Documento, origem, destino, responsáveis, aceite, recusa e motivo da divergência permanecem auditáveis.

## Regras invariantes

- Nenhuma autorização depende apenas da interface; o backend aplica perfil e unidade.
- Um solicitante não revisa a própria solicitação.
- Estoque disponível é o saldo físico menos a quantidade reservada.
- Compra aprovada não aumenta o estoque antes do recebimento.
- Transferência aprovada não credita o destino antes do recebimento.
- Saldo ou ativo reservado não pode ser consumido por movimentação comum.
- Um ativo em trânsito só pode ser alterado pelo recebimento do despacho correspondente.
- Toda decisão relevante registra identificadores, responsável, data e dados anteriores/posteriores na auditoria.
- Administradores não precisam de vínculos individuais porque seu escopo é global; operadores e gestores precisam de pelo menos uma unidade ativa.
