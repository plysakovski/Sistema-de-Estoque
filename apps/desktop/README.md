# StockManager Pro Desktop

Nova base profissional do sistema de estoque, construída para funcionar como aplicativo local instalável, sem depender de servidor web ou planilhas em tempo de execução.

O núcleo atual trabalha com dois modelos no mesmo catálogo:

- produtos controlados por quantidade e saldo por unidade;
- ativos individualizados por patrimônio e número de série.

Entradas, saídas, transferências e ajustes são transacionais, atualizam o dashboard e geram histórico e auditoria.

Unidades e categorias possuem cadastro, edição, ativação e desativação segura. Registros com estoque, ativos ou produtos vinculados não podem ser desativados acidentalmente.

A tela de Ativos oferece busca e filtros por unidade/estado, resumo operacional e atualização transacional de localização e condição. Equipamentos baixados ficam disponíveis apenas para consulta e só podem ser baixados pelo fluxo formal de saída.

A Auditoria apresenta até 500 eventos recentes em modo somente leitura, com busca, filtros por entidade, ação e período, além da comparação detalhada entre os dados anteriores e posteriores.

Configurações permite criar backups locais, consultar sua integridade e restaurar pontos anteriores. Toda restauração aceita somente arquivos da pasta interna, valida o SQLite e cria uma cópia de emergência antes de substituir o estado atual.

A importação aceita planilhas Excel/ODS e CSV. O sistema reconhece tanto o formato tabular atual quanto as abas do protótipo legado, apresenta erros por linha sem gravar dados e só confirma uma prévia integralmente válida. A confirmação cria um backup `pre-import-*`, grava produtos, ativos, saldos, movimentos e auditoria em uma única transação.

## Tecnologias

- TypeScript + React + Vite para a interface.
- Tauri 2 para empacotamento desktop e comunicação segura.
- Rust para regras de negócio, leitura de planilhas, persistência, auditoria e backup.
- SQLite local com chaves estrangeiras, WAL, índices e migrações.
- TanStack Query e Zod para cache, sincronização e validação de contratos.

## Desenvolvimento rápido da interface

```powershell
pnpm install
pnpm dev
```

No navegador, a aplicação usa um adaptador de demonstração. No aplicativo Tauri, a mesma interface troca automaticamente para o SQLite real.

## Aplicativo desktop

Pré-requisitos no Windows: Rust stable com MSVC, Microsoft C++ Build Tools e WebView2.

```powershell
pnpm install
pnpm tauri dev
```

Para gerar instaladores:

```powershell
pnpm.cmd desktop:build
```

O instalador será gerado no cache de build do usuário. Para entregas internas, copie o artefato final para `release/` e registre seu SHA-256 no relatório da versão em `../../docs/security/`.

O instalador NSIS (`.exe`) será gerado em `src-tauri/target/release/bundle/nsis`. Esse formato foi escolhido como padrão no Windows porque é direto para distribuir e não depende do empacotador WiX/MSI.

## Qualidade

```powershell
pnpm check
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

Para entender o projeto arquivo por arquivo, leia o [Guia do código](../../docs/GUIA_DO_CODIGO.md).

Antes de ampliar os módulos, consulte também a [Arquitetura](../../docs/ARCHITECTURE.md).
