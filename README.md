# StockManager Pro

> Functional preview of a web-based inventory management system built for enterprise environments. Runs entirely locally via a lightweight Python HTTP server — no cloud, no database, no installation required.

![Stack](https://img.shields.io/badge/stack-HTML%20%7C%20Vanilla%20JS%20%7C%20Python%203-4caf50?style=flat-square)
![Status](https://img.shields.io/badge/status-functional%20preview-e67e22?style=flat-square)
![License](https://img.shields.io/badge/license-MIT-3498db?style=flat-square)

---

## Overview

StockManager Pro is a single-file web application that reads and writes directly to `.xlsx` spreadsheets stored on a local or shared network drive. It was built for teams that need a quick, portable inventory solution without setting up a database or paying for SaaS tools.

---

## Features

- **Multi-sheet inventory** — manage multiple units, locations, or categories across two spreadsheets
- **Global search** — search across all sheets simultaneously with multi-term support (use `;` as separator)
- **Smart transfer** — move items between sheets with automatic field mapping
- **Column filters** — filter by any column with persistent state (saved when navigating away)
- **Alphabetical sorting** — click any column header to sort; state is preserved per sheet
- **PDF & DOCX reports** — generate formatted reports from any filtered view
- **Audit logs** — every action is logged with timestamp and exportable to `.xlsx`
- **Row tagging** — pin rows to top, assign colors, and mark storage location (Estoque A / Estoque B)
- **Expandable OBS field** — long observation notes collapse by default and expand on click
- **Date formatting** — Excel serial dates (e.g. `46150`) are automatically converted to `DD/MM/YYYY`
- **Column management** — add, remove, reorder, and rename columns on any sheet
- **Auto-save** — changes are saved back to the spreadsheet automatically via the local server

---

## Project Structure

```
📁 StockManager_Pro/
  ├── StockManager_Pro.html     # Full application (single file)
  ├── server_commercial.py      # Lightweight Python HTTP server
  ├── iniciar_commercial.bat    # Windows launcher
  ├── unidades.xlsx             # Spreadsheet: units / locations
  └── categorias.xlsx           # Spreadsheet: inventory categories
```

---

## Getting Started

### Requirements

- Python 3.x (no extra packages needed)
- A modern browser (Chrome, Edge, Firefox)

### Running

1. Clone or download this repository
2. Place your spreadsheet files in the same folder (or use the provided examples)
3. Double-click `iniciar_commercial.bat`
4. The browser will open automatically at `http://localhost:8000`

```bash
# Or run manually:
python server_commercial.py
```

---

## Spreadsheet Structure

### `unidades.xlsx`
One sheet per unit or location. Each sheet contains:

| Equipamento | Modelo | Service Tag | Responsavel | Local | Obs |
|---|---|---|---|---|---|

### `categorias.xlsx`
One sheet per equipment category. Included by default:

| Sheet | Key Columns |
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

You can rename, add, or remove sheets directly from the `[abas]` panel inside the app.

---

## Column Presets

When creating or editing sheets, columns are selected from a standardized list:

`MODELO` `EQUIPAMENTO` `GUICHE` `IMEI` `IMEI/SERVICE TAG` `SERVICE TAG` `NUMERO SERIE` `OBS` `RESPONSÁVEL` `DATA` `DATA DEPOSITO` `DATA ESTOQUE` `EMPRESA` `PLACA VIATURA` `NUMERO CHAMADO` `IP`

---

## Tech Stack

| Layer | Technology |
|---|---|
| Frontend | HTML5 + Vanilla JavaScript |
| Server | Python 3 `http.server` |
| Spreadsheet I/O | [SheetJS (XLSX.js) 0.18.5](https://sheetjs.com/) |
| PDF generation | [jsPDF 2.5.1](https://github.com/parallax/jsPDF) + [jsPDF-AutoTable](https://github.com/simonbengtsson/jsPDF-AutoTable) |
| DOCX generation | [docx 7.8.2](https://github.com/dolanmiu/docx) |

---

## Limitations

- **Single user** — no real-time sync between multiple users; last write wins
- **No authentication** — anyone with network access to port 8000 can read/write
- **File size** — performance may degrade with very large spreadsheets (1000+ rows per sheet)
- **Local network only** — not designed for internet-facing deployment

> ⚠️ This is a **functional preview**, not a production-ready system. Designed for internal validation, small teams, and stakeholder demonstration.

---

## Roadmap

- [ ] Multi-user conflict detection
- [ ] QR code label printing
- [ ] Barcode scanner input support
- [ ] Dark/light theme toggle
- [ ] CSV import

---

## License

MIT — free to use, modify, and distribute.
