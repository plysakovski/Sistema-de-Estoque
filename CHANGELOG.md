# Histórico de mudanças

As mudanças relevantes deste projeto são registradas neste arquivo. O formato segue [Keep a Changelog](https://keepachangelog.com/pt-BR/1.1.0/) e o versionamento segue [SemVer](https://semver.org/lang/pt-BR/).

## Não publicado

### Alterado

- Repositório reorganizado em aplicativo desktop, documentação e protótipo legado.
- Documentação pública, política de segurança e automação de qualidade profissionalizadas.

## 1.8.1 - 2026-09-01

### Adicionado

- Recebimento em lote vinculado à solicitação de reposição aprovada.
- Preenchimento automático dos produtos e quantidades ainda pendentes.
- Recebimento parcial explícito para produtos quantitativos e ativos serializados.
- Progresso recebido/aprovado na listagem de reposições.

### Segurança

- Validação no backend contra produto externo, quantidade excessiva, unidade divergente e pedido em estado inválido.
- Obrigatoriedade de documento ou referência no recebimento vinculado.

## 1.8.0 - 2026-08-31

### Adicionado

- Movimentações em lote.
- Catálogo separado do recebimento físico.
- Lotes e ativos rastreáveis por patrimônio e número de série.
- Ocorrências de ativos e fluxo gerencial de manutenção.

## 1.7.1 - 2026-08-30

### Corrigido

- Transições operacionais de ativos permitidas ao operador dentro das unidades vinculadas.

## 1.7.0 - 2026-08-29

### Adicionado

- Autenticação local e perfis operador, gestor e administrador.
- Escopo de acesso por unidade.
- Solicitações auditáveis de ajuste e reposição.
