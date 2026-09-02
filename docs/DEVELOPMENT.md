# Desenvolvimento e entrega

## Estrutura de trabalho

- Uma feature não consulta Tauri diretamente; usa um gateway de `infrastructure`.
- Tipos compartilhados entram em `domain`, sem dependência de React.
- Componentes visuais pequenos ficam em `components`; componentes específicos permanecem dentro da feature.
- Evite arquivos agregadores `index.ts` para manter importações e builds leves.
- Consultas independentes devem ser iniciadas em paralelo.

## Migrações

Nunca altere uma migração já distribuída. Crie `0002_nome.sql`, registre a versão e aplique em ordem durante a inicialização. Faça backup antes de migrações destrutivas.

## Importação das planilhas atuais

O importador implementado é um fluxo explícito e reversível:

1. selecionar uma planilha e a unidade de destino;
2. validar cabeçalhos e apresentar uma prévia;
3. normalizar categorias/unidades e apontar duplicidades;
4. criar um backup automático;
5. importar em uma única transação;
6. mostrar totais aceitos, rejeitados e motivos;
7. registrar a operação em auditoria.

As planilhas originais permanecem apenas como fonte de migração e exportação, não como banco ativo.

O núcleo aceita XLSX, XLS, XLSB, ODS e CSV UTF-8, limita arquivos a 5 MB/5.000 linhas e não atualiza SKUs existentes silenciosamente. Novos formatos devem ser adicionados como adaptadores em `imports.rs`, reutilizando a validação comum.

## Instalação no Windows

1. Instale Rust stable/MSVC e os pré-requisitos do Tauri.
2. Execute `pnpm install`.
3. Rode `pnpm check` e os testes Rust.
4. No Windows, execute `pnpm.cmd desktop:build`. O script verifica e prepara a ferramenta nativa exigida pelo SQLCipher e usa uma pasta de compilação curta.
5. Assine o instalador para distribuição externa e mantenha a chave fora do repositório.

O SQLite é embutido no binário por `rusqlite/bundled`; o usuário final não precisa instalar banco de dados ou Python.
