# Política de segurança

## Versões acompanhadas

O projeto está em desenvolvimento e recebe correções somente na versão mais recente da branch principal. Builds de demonstração e o protótipo em `legacy/` não devem ser usados com dados reais ou expostos à internet.

## Reportar uma vulnerabilidade

Não publique vulnerabilidades, credenciais, bancos ou dados pessoais em issues. Utilize o recurso **Report a vulnerability** da aba **Security** do repositório (GitHub Private Vulnerability Reporting), quando habilitado, ou entre em contato privado com o mantenedor pelo perfil do repositório.

Inclua, quando possível:

- versão e ambiente afetados;
- descrição do impacto;
- passos mínimos para reprodução;
- evidências sem dados reais;
- sugestão de mitigação.

O recebimento será confirmado em até 5 dias úteis. A correção, a divulgação coordenada e eventual crédito serão definidos conforme impacto e complexidade.

## Escopo prioritário

Autenticação, autorização por unidade, criptografia, importação de arquivos, backup/restauração, comandos Tauri, SQL, auditoria e atualização do aplicativo são considerados superfícies críticas.

Consulte a [política técnica](docs/SECURITY.md) e a [revisão da versão 1.9.0](docs/security/SECURITY_REVIEW_1.9.0.md).
