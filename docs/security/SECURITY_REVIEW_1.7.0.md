# Revisão de segurança — 1.7.0

Data: 31 de agosto de 2026

## Escopo

- Autenticação e autorização no backend Rust.
- Isolamento de dados por unidade.
- Solicitações de ajuste e reposição.
- Banco SQLCipher, credenciais e auditoria.
- Dependências TypeScript e Rust.
- Configuração Tauri e processo de distribuição Windows.

## Controles confirmados

- Senhas protegidas com Argon2id e apagadas da memória intermediária com `Zeroizing`.
- Sessões com expiração por inatividade, limite absoluto e invalidação por versão.
- Bloqueio temporário após falhas de autenticação.
- Banco SQLCipher com chave de 256 bits mantida pelo Gerenciador de Credenciais do Windows.
- CSP restritiva e capacidade Tauri limitada à janela principal, APIs básicas e log.
- Consultas e mutações de estoque, ativos, movimentos, ajustes, reposições e dashboard limitadas no backend pelas unidades do usuário.
- Separação entre solicitante e revisor.
- Reserva transacional para impedir dupla promessa de saldo em transferências aprovadas.
- Auditoria das solicitações e decisões com IDs dos responsáveis.
- Aplicativo de produção configurado como subsistema gráfico do Windows, sem console auxiliar.

## Varredura de dependências

- `pnpm audit --prod`: nenhuma vulnerabilidade conhecida.
- `cargo audit`: nenhuma vulnerabilidade bloqueante; 17 avisos permitidos de manutenção/solidez em dependências transitivas.
- Os avisos GTK3 e `glib` pertencem ao grafo Linux e não integram o alvo Windows verificado.
- Os pacotes `unic-*` chegam por `urlpattern`/Tauri no alvo Windows. São avisos de projeto sem manutenção, não uma vulnerabilidade explorável publicada. Risco residual baixo; acompanhar atualização do Tauri e repetir a análise em toda versão.

## Varredura de código e configuração

- Nenhuma chave de API, chave privada ou segredo fixo foi encontrado no código de produção.
- Ocorrências de `password` correspondem a tipos, formulários protegidos, processamento com `Zeroizing` e dados exclusivos de teste.
- Não existem permissões Tauri de shell, sistema de arquivos amplo ou execução de processos expostas ao frontend.
- A política CSP não permite scripts remotos.

## Riscos residuais e próximos controles

1. A assinatura Authenticode ainda depende da aquisição e proteção operacional de um certificado de assinatura de código.
2. O despacho/recebimento de reposição deve preservar reserva, trânsito e confirmação por usuários distintos quando configurado.
3. Transferências de ativos serializados devem exigir seleção individual e reserva por patrimônio antes de serem liberadas.
4. Para ambientes governamentais, recomenda-se política formal de rotação de credenciais, retenção de auditoria, cópia externa cifrada e homologação em máquina endurecida.
5. A auditoria de dependências deve continuar obrigatória antes de cada instalador assinado.

## Resultado

Versão aprovada para continuidade de desenvolvimento e homologação interna. Não liberar como versão final para ambiente crítico até concluir Authenticode, despacho/recebimento, transferência serializada e ensaio operacional de recuperação de desastre.
