use std::{
    collections::{HashMap, HashSet},
    io::Cursor,
};

use calamine::{open_workbook_auto_from_rs, Data, Reader};
use parking_lot::Mutex;
use serde::Serialize;
use uuid::Uuid;

use crate::error::{AppError, AppResult};

const MAX_FILE_SIZE: usize = 5 * 1024 * 1024;
const MAX_ROWS: usize = 5_000;

#[derive(Debug, Clone)]
pub struct ImportContext {
    pub existing_skus: HashSet<String>,
    pub existing_asset_tags: HashSet<String>,
    pub existing_serial_numbers: HashSet<String>,
}

#[derive(Debug, Clone)]
pub struct StagedAsset {
    pub asset_tag: String,
    pub serial_number: Option<String>,
}

#[derive(Debug, Clone)]
pub struct StagedProduct {
    pub sku: String,
    pub name: String,
    pub category: String,
    pub tracking_type: String,
    pub minimum_quantity: i64,
    pub quantity: i64,
    pub assets: Vec<StagedAsset>,
}

#[derive(Debug, Clone)]
pub struct StagedImport {
    pub file_name: String,
    pub location_id: String,
    pub products: Vec<StagedProduct>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportPreviewRow {
    pub row_number: usize,
    pub sheet: String,
    pub sku: String,
    pub name: String,
    pub category: String,
    pub tracking_type: String,
    pub quantity: i64,
    pub asset_tag: Option<String>,
    pub errors: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportPreview {
    pub token: Option<String>,
    pub file_name: String,
    pub source_mode: String,
    pub total_rows: usize,
    pub valid_rows: usize,
    pub error_rows: usize,
    pub products_to_create: usize,
    pub assets_to_create: usize,
    pub total_quantity: i64,
    pub general_errors: Vec<String>,
    pub rows: Vec<ImportPreviewRow>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub file_name: String,
    pub products_created: usize,
    pub assets_created: usize,
    pub quantity_imported: i64,
    pub safety_backup: String,
    pub imported_at: String,
}

#[derive(Debug, Clone)]
struct CandidateRow {
    row_number: usize,
    sheet: String,
    sku: String,
    name: String,
    category: String,
    tracking_type: String,
    minimum_quantity: i64,
    quantity: i64,
    asset_tag: Option<String>,
    serial_number: Option<String>,
    errors: Vec<String>,
}

#[derive(Default)]
pub struct ImportService {
    stages: Mutex<HashMap<String, StagedImport>>,
}

impl ImportService {
    pub fn preview(
        &self,
        file_name: &str,
        bytes: Vec<u8>,
        location_id: &str,
        context: ImportContext,
    ) -> AppResult<ImportPreview> {
        validate_file(file_name, &bytes)?;
        let (mut candidates, source_mode, mut general_errors) = parse_file(file_name, bytes)?;
        if candidates.len() > MAX_ROWS {
            return Err(AppError::Validation(format!(
                "A planilha excede o limite de {MAX_ROWS} linhas"
            )));
        }
        if candidates.is_empty() && general_errors.is_empty() {
            general_errors.push("Nenhuma linha de estoque foi encontrada".into());
        }

        validate_candidates(&mut candidates, &context);
        let valid_rows = candidates
            .iter()
            .filter(|row| row.errors.is_empty())
            .count();
        let error_rows = candidates.len() - valid_rows;
        let products = build_products(&candidates);
        let products_to_create = products.len();
        let assets_to_create = products.iter().map(|product| product.assets.len()).sum();
        let total_quantity = products.iter().map(|product| product.quantity).sum();
        let can_confirm = error_rows == 0 && general_errors.is_empty() && !products.is_empty();
        let token = can_confirm.then(|| Uuid::new_v4().to_string());

        if let Some(stage_token) = token.as_ref() {
            let mut stages = self.stages.lock();
            if stages.len() >= 3 {
                stages.clear();
            }
            stages.insert(
                stage_token.clone(),
                StagedImport {
                    file_name: file_name.to_owned(),
                    location_id: location_id.to_owned(),
                    products,
                },
            );
        }

        Ok(ImportPreview {
            token,
            file_name: file_name.to_owned(),
            source_mode,
            total_rows: candidates.len(),
            valid_rows,
            error_rows,
            products_to_create,
            assets_to_create,
            total_quantity,
            general_errors,
            rows: candidates
                .into_iter()
                .map(|row| ImportPreviewRow {
                    row_number: row.row_number,
                    sheet: row.sheet,
                    sku: row.sku,
                    name: row.name,
                    category: row.category,
                    tracking_type: row.tracking_type,
                    quantity: row.quantity,
                    asset_tag: row.asset_tag,
                    errors: row.errors,
                })
                .collect(),
        })
    }

    pub fn stage(&self, token: &str) -> AppResult<StagedImport> {
        self.stages.lock().get(token).cloned().ok_or_else(|| {
            AppError::Validation("A prévia expirou. Analise o arquivo novamente".into())
        })
    }

    pub fn discard(&self, token: &str) {
        self.stages.lock().remove(token);
    }
}

fn validate_file(file_name: &str, bytes: &[u8]) -> AppResult<()> {
    if bytes.is_empty() {
        return Err(AppError::Validation("O arquivo está vazio".into()));
    }
    if bytes.len() > MAX_FILE_SIZE {
        return Err(AppError::Validation(
            "O arquivo deve ter no máximo 5 MB".into(),
        ));
    }
    let extension = file_name
        .rsplit('.')
        .next()
        .unwrap_or_default()
        .to_lowercase();
    if !matches!(extension.as_str(), "xlsx" | "xls" | "xlsb" | "ods" | "csv") {
        return Err(AppError::Validation(
            "Use um arquivo XLSX, XLS, XLSB, ODS ou CSV".into(),
        ));
    }
    Ok(())
}

fn parse_file(
    file_name: &str,
    bytes: Vec<u8>,
) -> AppResult<(Vec<CandidateRow>, String, Vec<String>)> {
    if file_name.to_lowercase().ends_with(".csv") {
        let text = String::from_utf8(bytes)
            .map_err(|_| AppError::Validation("O CSV precisa estar codificado em UTF-8".into()))?;
        let rows = parse_csv(&text)?;
        let candidates = parse_standard_sheet("CSV", &rows).ok_or_else(|| {
            AppError::Validation("O CSV não possui os cabeçalhos SKU e NOME".into())
        })?;
        return Ok((candidates, "standard".into(), Vec::new()));
    }

    let cursor = Cursor::new(bytes);
    let mut workbook = open_workbook_auto_from_rs(cursor).map_err(|error| {
        AppError::Validation(format!("Não foi possível ler a planilha: {error}"))
    })?;
    let sheet_names = workbook.sheet_names().to_vec();
    let mut candidates = Vec::new();
    let mut used_standard = false;
    let mut used_legacy = false;
    let mut errors = Vec::new();

    for sheet_name in sheet_names {
        if should_skip_legacy_sheet(&sheet_name) {
            continue;
        }
        let range = match workbook.worksheet_range(&sheet_name) {
            Ok(range) => range,
            Err(error) => {
                errors.push(format!("Aba {sheet_name}: {error}"));
                continue;
            }
        };
        let rows: Vec<Vec<String>> = range
            .rows()
            .map(|row| row.iter().map(cell_text).collect())
            .collect();
        if let Some(mut parsed) = parse_standard_sheet(&sheet_name, &rows) {
            used_standard = true;
            candidates.append(&mut parsed);
        } else if let Some(mut parsed) = parse_legacy_sheet(&sheet_name, &rows) {
            used_legacy = true;
            candidates.append(&mut parsed);
        }
    }
    let mode = match (used_standard, used_legacy) {
        (true, true) => "mixed",
        (true, false) => "standard",
        (false, true) => "legacy",
        (false, false) => "unknown",
    };
    Ok((candidates, mode.into(), errors))
}

fn cell_text(cell: &Data) -> String {
    match cell {
        Data::Empty => String::new(),
        Data::Float(value) if value.fract() == 0.0 => format!("{value:.0}"),
        other => other.to_string().trim().to_owned(),
    }
}

fn parse_standard_sheet(sheet: &str, rows: &[Vec<String>]) -> Option<Vec<CandidateRow>> {
    let (header_index, headers) = rows.iter().take(10).enumerate().find_map(|(index, row)| {
        let normalized: Vec<String> = row.iter().map(|value| normalize(value)).collect();
        let has_sku = normalized
            .iter()
            .any(|value| matches!(value.as_str(), "sku" | "codigo" | "codigo_produto"));
        let has_name = normalized
            .iter()
            .any(|value| matches!(value.as_str(), "nome" | "produto" | "modelo"));
        (has_sku && has_name).then_some((index, normalized))
    })?;
    let index = |aliases: &[&str]| {
        headers
            .iter()
            .position(|header| aliases.contains(&header.as_str()))
    };
    let sku = index(&["sku", "codigo", "codigo_produto"])?;
    let name = index(&["nome", "produto", "modelo"])?;
    let category = index(&["categoria", "grupo"]);
    let tracking = index(&["controle", "tipo", "tipo_controle"]);
    let quantity = index(&["quantidade", "qtd", "saldo"]);
    let minimum = index(&["minimo", "estoque_minimo", "quantidade_minima"]);
    let asset_tag = index(&["patrimonio", "asset_tag", "service_tag", "imei"]);
    let serial = index(&["serie", "numero_serie", "serial"]);
    let mut result = Vec::new();

    for (row_index, row) in rows.iter().enumerate().skip(header_index + 1) {
        if row.iter().all(|value| value.trim().is_empty()) {
            continue;
        }
        let value = |column: usize| row.get(column).map(|item| item.trim()).unwrap_or_default();
        let tag = asset_tag
            .map(|column| value(column).to_owned())
            .filter(|value| !value.is_empty());
        let tracking_type = tracking
            .map(|column| parse_tracking_type(value(column)))
            .unwrap_or_else(|| {
                if tag.is_some() {
                    "serialized"
                } else {
                    "quantity"
                }
                .into()
            });
        let parsed_quantity = quantity
            .map(|column| parse_integer(value(column), "Quantidade"))
            .unwrap_or(Ok(if tracking_type == "serialized" { 1 } else { 0 }));
        let parsed_minimum = minimum
            .map(|column| parse_integer(value(column), "Mínimo"))
            .unwrap_or(Ok(0));
        let mut errors = Vec::new();
        let quantity_value = parsed_quantity.unwrap_or_else(|error| {
            errors.push(error);
            0
        });
        let minimum_value = parsed_minimum.unwrap_or_else(|error| {
            errors.push(error);
            0
        });
        result.push(CandidateRow {
            row_number: row_index + 1,
            sheet: sheet.to_owned(),
            sku: value(sku).to_owned(),
            name: value(name).to_owned(),
            category: category
                .map(|column| value(column).to_owned())
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| sheet.to_owned()),
            tracking_type,
            minimum_quantity: minimum_value,
            quantity: quantity_value,
            asset_tag: tag,
            serial_number: serial
                .map(|column| value(column).to_owned())
                .filter(|value| !value.is_empty()),
            errors,
        });
    }
    Some(result)
}

fn parse_legacy_sheet(sheet: &str, rows: &[Vec<String>]) -> Option<Vec<CandidateRow>> {
    let (header_index, headers) = rows.iter().take(12).enumerate().find_map(|(index, row)| {
        let normalized: Vec<String> = row.iter().map(|value| normalize(value)).collect();
        let model = normalized
            .iter()
            .any(|value| value == "modelo" || value == "nome_do_equipamento");
        let identifier = normalized.iter().any(|value| is_identifier_header(value));
        (model && identifier).then_some((index, normalized))
    })?;
    let model_column = headers
        .iter()
        .position(|value| value == "modelo" || value == "nome_do_equipamento")?;
    let identifier_column = headers
        .iter()
        .position(|value| is_identifier_header(value))?;
    let mut result = Vec::new();
    for (row_index, row) in rows.iter().enumerate().skip(header_index + 1) {
        let model = row
            .get(model_column)
            .map(|value| value.trim())
            .unwrap_or_default();
        let identifier = row
            .get(identifier_column)
            .map(|value| value.trim())
            .unwrap_or_default();
        if model.is_empty() && identifier.is_empty() {
            continue;
        }
        result.push(CandidateRow {
            row_number: row_index + 1,
            sheet: sheet.to_owned(),
            sku: legacy_sku(sheet, model),
            name: model.to_owned(),
            category: sheet.to_owned(),
            tracking_type: "serialized".into(),
            minimum_quantity: 0,
            quantity: 1,
            asset_tag: (!identifier.is_empty()).then(|| identifier.to_owned()),
            serial_number: (!identifier.is_empty()).then(|| identifier.to_owned()),
            errors: Vec::new(),
        });
    }
    Some(result)
}

fn validate_candidates(rows: &mut [CandidateRow], context: &ImportContext) {
    let mut skus: HashMap<String, (String, String, String)> = HashMap::new();
    let mut tags = HashSet::new();
    let mut serials = HashSet::new();
    for row in rows {
        if row.sku.trim().len() < 2 || row.sku.len() > 64 {
            row.errors
                .push("SKU deve ter entre 2 e 64 caracteres".into());
        }
        if row.name.trim().len() < 2 || row.name.len() > 160 {
            row.errors
                .push("Nome deve ter entre 2 e 160 caracteres".into());
        }
        if row.category.trim().len() < 2 || row.category.len() > 80 {
            row.errors
                .push("Categoria deve ter entre 2 e 80 caracteres".into());
        }
        if !matches!(row.tracking_type.as_str(), "quantity" | "serialized") {
            row.errors
                .push("Controle deve ser quantidade ou serializado".into());
        }
        if row.quantity < 0 {
            row.errors.push("Quantidade não pode ser negativa".into());
        }
        if row.minimum_quantity < 0 {
            row.errors.push("Mínimo não pode ser negativo".into());
        }
        if row.tracking_type == "serialized" {
            if row.quantity != 1 {
                row.errors
                    .push("Cada linha serializada deve representar exatamente um ativo".into());
            }
            if row.asset_tag.as_deref().unwrap_or_default().trim().len() < 2 {
                row.errors
                    .push("Informe o patrimônio, Service Tag ou IMEI".into());
            }
        } else if row.asset_tag.is_some() {
            row.errors
                .push("Produto por quantidade não deve informar patrimônio".into());
        }

        let sku_key = normalize_key(&row.sku);
        if context.existing_skus.contains(&sku_key) {
            row.errors
                .push("SKU já cadastrado; a importação não sobrescreve produtos".into());
        }
        if let Some((tracking, name, category)) = skus.get(&sku_key) {
            if row.tracking_type == "quantity" || *tracking == "quantity" {
                row.errors
                    .push("SKU repetido em produto controlado por quantidade".into());
            } else if normalize_key(name) != normalize_key(&row.name)
                || normalize_key(category) != normalize_key(&row.category)
            {
                row.errors
                    .push("Linhas do mesmo SKU possuem nome ou categoria diferentes".into());
            }
        } else {
            skus.insert(
                sku_key,
                (
                    row.tracking_type.clone(),
                    row.name.clone(),
                    row.category.clone(),
                ),
            );
        }

        if let Some(tag) = row.asset_tag.as_ref() {
            let key = normalize_key(tag);
            if context.existing_asset_tags.contains(&key) {
                row.errors.push("Patrimônio já cadastrado".into());
            }
            if !tags.insert(key) {
                row.errors.push("Patrimônio repetido no arquivo".into());
            }
        }
        if let Some(serial) = row.serial_number.as_ref() {
            let key = normalize_key(serial);
            if context.existing_serial_numbers.contains(&key) {
                row.errors.push("Número de série já cadastrado".into());
            }
            if !serials.insert(key) {
                row.errors
                    .push("Número de série repetido no arquivo".into());
            }
        }
    }
}

fn build_products(rows: &[CandidateRow]) -> Vec<StagedProduct> {
    let mut products: Vec<StagedProduct> = Vec::new();
    for row in rows.iter().filter(|row| row.errors.is_empty()) {
        if let Some(product) = products
            .iter_mut()
            .find(|product| normalize_key(&product.sku) == normalize_key(&row.sku))
        {
            if let Some(tag) = row.asset_tag.as_ref() {
                product.assets.push(StagedAsset {
                    asset_tag: tag.clone(),
                    serial_number: row.serial_number.clone(),
                });
                product.quantity += 1;
            }
            continue;
        }
        products.push(StagedProduct {
            sku: row.sku.trim().to_owned(),
            name: row.name.trim().to_owned(),
            category: row.category.trim().to_owned(),
            tracking_type: row.tracking_type.clone(),
            minimum_quantity: row.minimum_quantity,
            quantity: row.quantity,
            assets: row
                .asset_tag
                .as_ref()
                .map(|tag| {
                    vec![StagedAsset {
                        asset_tag: tag.clone(),
                        serial_number: row.serial_number.clone(),
                    }]
                })
                .unwrap_or_default(),
        });
    }
    products
}

fn parse_csv(text: &str) -> AppResult<Vec<Vec<String>>> {
    let delimiter = if text.lines().next().unwrap_or_default().matches(';').count()
        > text.lines().next().unwrap_or_default().matches(',').count()
    {
        ';'
    } else {
        ','
    };
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let mut chars = text.trim_start_matches('\u{feff}').chars().peekable();
    while let Some(character) = chars.next() {
        match character {
            '"' if quoted && chars.peek() == Some(&'"') => {
                field.push('"');
                chars.next();
            }
            '"' => quoted = !quoted,
            value if value == delimiter && !quoted => {
                row.push(std::mem::take(&mut field));
            }
            '\n' if !quoted => {
                row.push(std::mem::take(&mut field));
                rows.push(std::mem::take(&mut row));
            }
            '\r' if !quoted => {}
            value => field.push(value),
        }
    }
    if quoted {
        return Err(AppError::Validation("CSV com aspas não finalizadas".into()));
    }
    if !field.is_empty() || !row.is_empty() {
        row.push(field);
        rows.push(row);
    }
    Ok(rows)
}

fn parse_integer(value: &str, label: &str) -> Result<i64, String> {
    if value.trim().is_empty() {
        return Ok(0);
    }
    value
        .trim()
        .parse::<i64>()
        .map_err(|_| format!("{label} deve ser um número inteiro"))
}

fn parse_tracking_type(value: &str) -> String {
    match normalize(value).as_str() {
        "serializado" | "serial" | "patrimonial" | "ativo" | "serialized" => "serialized".into(),
        "quantidade" | "saldo" | "quantity" => "quantity".into(),
        other => other.to_owned(),
    }
}

fn is_identifier_header(value: &str) -> bool {
    matches!(
        value,
        "service_tag"
            | "service_tag_imei"
            | "imei"
            | "patrimonio"
            | "numero_serie"
            | "n_serie"
            | "serial"
    )
}

fn should_skip_legacy_sheet(sheet: &str) -> bool {
    let value = normalize(sheet);
    value.starts_with("_bd_")
        || matches!(value.as_str(), "enviado_para" | "descartados" | "baixados")
}

fn legacy_sku(category: &str, model: &str) -> String {
    let category_code = slug(category);
    let model_code = slug(model);
    let hash = model.bytes().fold(2_166_136_261_u32, |value, byte| {
        (value ^ u32::from(byte)).wrapping_mul(16_777_619)
    });
    format!(
        "LEG-{}-{}-{hash:08X}",
        truncate(&category_code, 18),
        truncate(&model_code, 25)
    )
}

fn truncate(value: &str, maximum: usize) -> &str {
    value.get(..maximum).unwrap_or(value)
}

fn slug(value: &str) -> String {
    let normalized = normalize(value).to_uppercase();
    let mut result = String::new();
    for character in normalized.chars() {
        if character.is_ascii_alphanumeric() {
            result.push(character);
        } else if !result.ends_with('-') {
            result.push('-');
        }
    }
    result.trim_matches('-').to_owned()
}

pub fn normalize_key(value: &str) -> String {
    normalize(value)
        .replace('_', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn normalize(value: &str) -> String {
    value
        .trim()
        .to_lowercase()
        .replace(['á', 'à', 'ã', 'â', 'ä'], "a")
        .replace(['é', 'è', 'ê', 'ë'], "e")
        .replace(['í', 'ì', 'î', 'ï'], "i")
        .replace(['ó', 'ò', 'õ', 'ô', 'ö'], "o")
        .replace(['ú', 'ù', 'û', 'ü'], "u")
        .replace('ç', "c")
        .replace(['/', '-', ' '], "_")
        .split('_')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("_")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_context() -> ImportContext {
        ImportContext {
            existing_skus: HashSet::new(),
            existing_asset_tags: HashSet::new(),
            existing_serial_numbers: HashSet::new(),
        }
    }

    #[test]
    fn previews_valid_csv_without_persisting_it() {
        let service = ImportService::default();
        let csv = "SKU;NOME;CATEGORIA;CONTROLE;QUANTIDADE;MINIMO;PATRIMONIO;SERIE\nCABO-01;Cabo HDMI;Cabos;quantidade;12;3;;\nNOTE-01;Notebook Dell;Notebooks;serializado;1;0;PAT-901;SN-901";
        let preview = service
            .preview(
                "estoque.csv",
                csv.as_bytes().to_vec(),
                "location",
                empty_context(),
            )
            .expect("preview");
        assert_eq!(preview.total_rows, 2);
        assert_eq!(preview.products_to_create, 2);
        assert_eq!(preview.assets_to_create, 1);
        assert_eq!(preview.total_quantity, 13);
        assert!(preview.token.is_some());
    }

    #[test]
    fn blocks_duplicate_and_existing_identifiers() {
        let service = ImportService::default();
        let csv = "SKU,NOME,CATEGORIA,CONTROLE,QUANTIDADE,PATRIMONIO\nNOTE-01,Notebook,Notebooks,serializado,1,PAT-1\nNOTE-01,Notebook,Notebooks,serializado,1,PAT-1";
        let mut context = empty_context();
        context.existing_skus.insert(normalize_key("NOTE-01"));
        let preview = service
            .preview("estoque.csv", csv.as_bytes().to_vec(), "location", context)
            .expect("preview");
        assert_eq!(preview.error_rows, 2);
        assert!(preview.token.is_none());
        assert!(preview.rows.iter().any(|row| row
            .errors
            .iter()
            .any(|error| error.contains("já cadastrado"))));
    }

    #[test]
    fn converts_legacy_sheet_into_serialized_products() {
        let rows = vec![
            vec!["COMPUTADORES".into(), "".into()],
            vec!["MODELO".into(), "SERVICE TAG".into()],
            vec!["Dell Optiplex 3000".into(), "ERSA1111".into()],
            vec!["Dell Optiplex 3000".into(), "ERSA2222".into()],
        ];
        let mut candidates = parse_legacy_sheet("Computadores", &rows).expect("legacy");
        validate_candidates(&mut candidates, &empty_context());
        let products = build_products(&candidates);
        assert_eq!(products.len(), 1);
        assert_eq!(products[0].assets.len(), 2);
        assert_eq!(products[0].quantity, 2);
    }

    #[test]
    fn rejects_unclosed_csv_quotes() {
        let error = parse_csv("SKU,NOME\n1,\"aberto").expect_err("invalid CSV");
        assert!(error.to_string().contains("aspas"));
    }

    #[test]
    fn reads_reference_legacy_workbook_when_available() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../categorias.xlsx");
        if !path.exists() {
            return;
        }
        let service = ImportService::default();
        let preview = service
            .preview(
                "categorias.xlsx",
                std::fs::read(path).expect("reference workbook"),
                "location",
                empty_context(),
            )
            .expect("legacy preview");
        assert_eq!(preview.source_mode, "legacy");
        assert!(preview.total_rows >= 10);
        assert!(preview.rows.iter().all(|row| row.sheet != "ENVIADO_PARA"));
    }
}
