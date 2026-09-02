# Segurança do StockManager Pro

## Objetivo

O StockManager Pro adota defesa em profundidade para operar com dados de empresas e órgãos públicos. Nenhuma decisão de autorização deve depender apenas da interface. Toda operação sensível é validada novamente no backend Rust e registrada em auditoria.

## Modelo de ameaças

O projeto considera, no mínimo:

- acesso físico ou lógico indevido à máquina e aos arquivos do aplicativo;
- usuário autenticado tentando executar uma função acima do próprio perfil;
- comprometimento da interface WebView e chamadas IPC forjadas;
- tentativa de adivinhar credenciais, enumerar usuários ou reutilizar sessões;
- adulteração de solicitações enquanto aguardam análise;
- extração do banco, backups ou arquivos temporários;
- dependências vulneráveis, segredos incluídos no código e configurações inseguras;
- perda da máquina ou da credencial do sistema operacional.

## Perfis e permissões

| Operação | Operador | Gestor | Administrador |
|---|---:|---:|---:|
| Consultar estoque, ativos e movimentações | Sim | Sim | Sim |
| Criar e acompanhar a própria solicitação de ajuste | Sim | Sim | Sim |
| Cadastrar itens e executar movimentações regulares | Não | Sim | Sim |
| Analisar solicitações pendentes | Não | Sim | Sim |
| Consultar auditoria completa | Não | Sim | Sim |
| Gerenciar locais, categorias, importações e backups | Não | Sim | Sim |
| Gerenciar usuários, perfis e parâmetros de segurança | Não | Não | Sim |
| Restaurar backup e executar recuperação criptográfica | Não | Não | Sim |

O administrador herda as permissões do gestor, e o gestor herda as permissões de consulta do operador. A autorização é explícita no backend; a hierarquia não é usada como substituto para a checagem da permissão concreta.

## Autenticação e sessões

- Senhas são armazenadas somente como hash Argon2id com salt exclusivo.
- A política aceita frases-senha longas e Unicode, com tamanho entre 12 e 128 caracteres.
- Erros de login não revelam se o usuário existe, está inativo ou informou senha incorreta.
- Tentativas malsucedidas geram auditoria e bloqueio temporário progressivo.
- A sessão existe apenas na memória protegida do processo Rust; a interface não recebe nem persiste senha ou token em `localStorage`.
- A sessão expira por inatividade e também possui duração máxima absoluta.
- Alterar senha, perfil ou situação do usuário invalida a sessão afetada.
- O sistema impede remover ou rebaixar o último administrador ativo.

## Solicitações de ajuste

Uma solicitação registra um identificador imutável, solicitante, alvo, justificativa, estado anterior, alteração pretendida, versão do registro e data. Somente uma solicitação pendente pode ser decidida.

Na aprovação, o backend confere novamente:

1. se o revisor possui permissão de gestor;
2. se o revisor não é o próprio solicitante;
3. se o alvo e sua versão ainda correspondem ao estado analisado;
4. se a alteração mantém todas as regras do domínio;
5. se a solicitação continua pendente.

A alteração e a decisão são gravadas na mesma transação. A auditoria vincula o ID da solicitação, o operador solicitante, o alvo, o gestor revisor, a decisão e a justificativa. Recusas também são preservadas.

## Criptografia e recuperação

- O banco SQLite e seus backups usam SQLCipher com chave aleatória de 256 bits.
- A cópia operacional da chave fica no cofre de credenciais do sistema operacional, nunca no diretório do banco, nos logs ou no código-fonte.
- A chave de recuperação só pode ser revelada por um administrador após redigitar a própria senha; revelação e confirmação de armazenamento externo são auditadas sem registrar o segredo.
- Em outra máquina, o arquivo temporário `stockmanager.recovery-key` é validado contra o banco, importado para o cofre do Windows e removido após o sucesso.
- Importações são validadas antes de entrar no banco e arquivos temporários têm vida curta.
- A migração de bancos antigos em texto claro só substitui o original depois de criar e validar a versão criptografada.

Sem a chave do cofre do sistema ou a chave de recuperação, os dados não poderão ser recuperados. Essa propriedade é intencional e precisa ser contemplada no plano de continuidade da organização.

O SQLCipher tenta proteger páginas sensíveis da memória, mas o Windows pode negar parte das chamadas `VirtualLock` por limite de cota. A chave mantida pelo código Rust usa memória zerável (`Zeroizing`), não é registrada em logs e nunca é persistida junto do banco. Ambientes de risco elevado devem complementar esses controles com BitLocker, proteção física e política de bloqueio da estação.

## Verificação obrigatória por versão

Cada versão publicável deve possuir um relatório em `docs/security/` contendo:

- revisão de mudanças no modelo de ameaças e nas permissões;
- testes de autorização para todos os comandos Tauri;
- testes de autenticação, bloqueio, expiração e invalidação de sessão;
- testes do fluxo de solicitação, concorrência e trilha de auditoria;
- `cargo test`, `cargo clippy`, testes do frontend e compilação de produção;
- auditoria das dependências Rust e JavaScript;
- busca por credenciais, chaves, tokens e dados pessoais no repositório;
- revisão de CSP, capacidades Tauri, logs, arquivos temporários e permissões locais;
- achados classificados por severidade, responsável, correção e risco residual.

Uma vulnerabilidade crítica ou alta sem mitigação bloqueia a publicação.
