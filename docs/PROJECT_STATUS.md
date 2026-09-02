# Estado do projeto

## Versão atual

**1.8.1 — demonstração técnica e homologação controlada.**

O fluxo principal de estoque, ativos, solicitações, recebimento, autenticação e auditoria está funcional. Isso não equivale ainda a uma liberação institucional: alguns controles dependem do ambiente de implantação e precisam ser concluídos antes de operar dados sensíveis em produção.

## Concluído

- Catálogo híbrido e saldos por unidade.
- Movimentações em lote e recebimento vinculado à reposição.
- Individualização por patrimônio, série e lote.
- Perfis operador, gestor e administrador com escopo por unidade.
- Ajustes e reposições com decisão segregada e auditoria.
- Ocorrências de ativos.
- Banco local criptografado, sessão local e proteção contra tentativas repetidas.
- Backup, restauração validada e importação com prévia.
- Instalador NSIS sem terminal auxiliar.
- Testes automatizados de domínio e backend.

## Próximas demandas

### Prioridade crítica — liberação institucional

- Assinar instalador e executável com certificado Authenticode.
- Definir gestão segura da chave de criptografia por organização e procedimento de recuperação.
- Concluir modelo de atualização assinada e política de rollback.
- Realizar threat modeling, análise de dependências e teste de invasão independente.
- Definir retenção, exportação, integridade e acesso aos logs de auditoria.
- Criar procedimento operacional de backup externo, restauração e teste periódico.

### Prioridade alta — regras de negócio

- Implementar transferência aprovada em duas etapas: despacho na origem e recebimento no destino.
- Concluir cancelamento administrativo de reposições com justificativa e auditoria.
- Criar políticas configuráveis para patrimônio e número de série por categoria/produto.
- Adicionar inventário físico e conciliação formal de divergências.
- Definir estados finais e prazos de atendimento para ocorrências e solicitações.

### Prioridade média — operação

- Busca global entre produtos, ativos, lotes, movimentações e solicitações.
- Relatórios e exportações respeitando perfil e unidade.
- Etiquetas com QR Code/código de barras e suporte a leitor.
- Notificações internas de solicitações e ocorrências pendentes.
- Filtros, paginação e desempenho para bases maiores.
- Melhorias de acessibilidade e testes completos por teclado.

### Engenharia e qualidade

- Ampliar testes de interface e fluxos completos no aplicativo Tauri.
- Automatizar auditoria de dependências e geração de SBOM por versão.
- Versionar releases somente após aprovação da integração contínua.
- Documentar instalação, atualização, recuperação e desinstalação para usuários finais.

## Critério para produção

Uma versão só deve ser declarada pronta para produção após: regras críticas implementadas; revisão de segurança sem achados bloqueantes; recuperação de backup ensaiada; artefatos assinados e reproduzíveis; documentação operacional aprovada; e homologação dos três perfis em ambiente representativo.
