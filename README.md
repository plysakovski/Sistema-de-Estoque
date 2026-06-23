# StockManager Pro

> Preview funcional de um sistema web de gerenciamento de estoque empresarial. Roda completamente de forma local via um servidor Python leve — sem nuvem, sem banco de dados, sem instalação.

![Stack](https://img.shields.io/badge/stack-HTML%20%7C%20Vanilla%20JS%20%7C%20Python%203-4caf50?style=flat-square)
![Status](https://img.shields.io/badge/status-preview%20funcional-e67e22?style=flat-square)
![Licença](https://img.shields.io/badge/licen%C3%A7a-MIT-3498db?style=flat-square)

---

## Visão Geral

O StockManager Pro é uma aplicação web em arquivo único que lê e escreve diretamente em planilhas `.xlsx` armazenadas em disco local ou unidade de rede compartilhada. Foi desenvolvido para equipes que precisam de uma solução de inventário portátil e rápida, sem necessidade de configurar banco de dados ou pagar por ferramentas SaaS.

---

## Funcionalidades

- **Inventário multi-planilha** — gerencie unidades, locais e categorias em duas planilhas separadas
- **Busca global** — pesquise em todas as abas simultaneamente com suporte a múltiplos termos (separados por `;`)
- **Transferência inteligente** — mova itens entre abas com preenchimento automático dos campos compatíveis
- **Filtros por coluna** — filtre por qualquer campo com estado persistente (mantido ao navegar entre abas)
- **Ordenação alfabética** — clique em qualquer cabeçalho de coluna para ordenar; estado salvo por aba
- **Relatórios PDF e DOCX** — gere relatórios formatados a partir de qualquer visualização filtrada
- **Logs de auditoria** — cada ação é registrada com data/hora e exportável para `.xlsx`
- **Marcação de linhas** — fixe linhas no topo, atribua cores e marque localização de estoque (Estoque A / Estoque B)
- **Campo OBS expansível** — observações longas ficam recolhidas e expandem ao clicar
- **Formatação de datas** — datas seriais do Excel (ex: `46150`) convertidas automaticamente para `DD/MM/AAAA`
- **Gerenciamento de colunas** — adicione, remova, reordene e renomeie colunas de qualquer aba
- **Salvar automático** — alterações são salvas de volta na planilha automaticamente via servidor local

---

## Estrutura do Projeto

```
📁 StockManager_Pro/
  ├── StockManager_Pro.html       # Aplicação completa (arquivo único)
  ├── server_commercial.py        # Servidor HTTP Python leve
  ├── iniciar_commercial.bat      # Iniciador para Windows
  ├── unidades.xlsx               # Planilha: unidades / locais
  └── categorias.xlsx             # Planilha: categorias do inventário
```

---

## Como Usar

### Requisitos

- Python 3.x (sem pacotes extras)
- Navegador moderno (Chrome, Edge ou Firefox)

### Executando

1. Clone ou baixe este repositório
2. Coloque os arquivos de planilha na mesma pasta (ou use os exemplos fornecidos)
3. Dê dois cliques em `iniciar_commercial.bat`
4. O navegador abrirá automaticamente em `http://localhost:8000`

```bash
# Ou execute manualmente:
python server_commercial.py
```

---

## Estrutura das Planilhas

### `unidades.xlsx`
Uma aba por unidade ou local. Cada aba contém:

| Equipamento | Modelo | Service Tag | Responsavel | Local | Obs |
|---|---|---|---|---|---|

### `categorias.xlsx`
Uma aba por categoria de equipamento. Incluídas por padrão:

| Aba | Colunas Principais |
|---|---|
| Computadores | MODELO / SERVICE TAG / NUMERO CHAMADO / OBS |
| Notebooks | MODELO / SERVICE TAG / NUMERO CHAMADO / OBS |
| Celulares | MODELO / IMEI / NUMERO CHAMADO / OBS |
| Tablets | MODELO / IMEI / NUMERO CHAMADO / OBS |
| Monitores | MODELO / SERVICE TAG / NUMERO CHAMADO / OBS |
| Perifericos | MODELO / SERVICE TAG / NUMERO CHAMADO / OBS |
| Impressoras | MODELO / SERVICE TAG / NUMERO CHAMADO / OBS |
| Defeito | STATUS / CHAMADO / SERVICE TAG / MODELO / TIPO / OBS |
| ENVIADO_PARA | NOME DO EQUIPAMENTO / MODELO / SERVICE TAG / DATA / CHAMADO / OBS |

Você pode renomear, adicionar ou remover abas diretamente pelo painel `[abas]` dentro do sistema.

---

## Colunas Padronizadas

Ao criar ou editar abas, as colunas são selecionadas a partir de uma lista padronizada:

`MODELO` `EQUIPAMENTO` `GUICHE` `IMEI` `IMEI/SERVICE TAG` `SERVICE TAG` `NUMERO SERIE` `OBS` `RESPONSÁVEL` `DATA` `DATA DEPOSITO` `DATA ESTOQUE` `EMPRESA` `PLACA VIATURA` `NUMERO CHAMADO` `IP`

---

## Tecnologias

| Camada | Tecnologia |
|---|---|
| Frontend | HTML5 + JavaScript puro |
| Servidor | Python 3 `http.server` |
| Leitura/escrita de planilhas | [SheetJS (XLSX.js) 0.18.5](https://sheetjs.com/) |
| Geração de PDF | [jsPDF 2.5.1](https://github.com/parallax/jsPDF) + [jsPDF-AutoTable](https://github.com/simonbengtsson/jsPDF-AutoTable) |
| Geração de DOCX | [docx 7.8.2](https://github.com/dolanmiu/docx) |

---

## Limitações

- **Usuário único** — sem sincronização em tempo real entre múltiplos usuários; o último a salvar prevalece
- **Sem autenticação** — qualquer pessoa com acesso à porta 8000 na rede pode ler e escrever
- **Tamanho do arquivo** — o desempenho pode cair com planilhas muito grandes (acima de 1000 linhas por aba)
- **Somente rede local** — não foi projetado para exposição à internet

> ⚠️ Este é um **preview funcional**, não um sistema pronto para produção. Desenvolvido para validação interna, equipes pequenas e demonstração para stakeholders.

---

## Melhorias Planejadas

- [ ] Detecção de conflito entre múltiplos usuários
- [ ] Impressão de etiquetas com QR Code
- [ ] Suporte a entrada via leitor de código de barras
- [ ] Importação de CSV
- [ ] Alternância entre tema claro e escuro

---

## Licença

MIT — livre para usar, modificar e distribuir.
