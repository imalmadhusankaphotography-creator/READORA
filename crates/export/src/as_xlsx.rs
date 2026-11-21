//! Export a PDF ▸ Excel (.xlsx): one worksheet per page. Each table the page's text layout forms
//! (the same detection the Word export uses) becomes a grid of cells, with cells that span grid
//! columns merged and thin borders where the page rules its grid; the page's other paragraphs
//! become one row each, in reading order, so nothing on the page is lost. Cells that hold a plain
//! number (`1,250.50`, `-3`, `(42)`, `12.5%`) are written as numbers, so they can be summed.

use super::layout::{Item, Table, items};
use super::{Page, Zip, esc};

/// Excel's limits: rows in a sheet, characters in a cell, characters in a sheet name.
const MAX_ROWS: usize = 1_048_576;
const MAX_CELL_CHARS: usize = 32_767;
const MAX_SHEET_NAME: usize = 31;
/// Column widths (in characters) a table's text may ask for.
const MIN_WIDTH: usize = 8;
const MAX_WIDTH: usize = 60;
/// Excel keeps 15 significant digits: a longer run of digits (an account or card number) stays text.
const MAX_DIGITS: usize = 15;

/// Cell styles (`cellXfs` in styles.xml): font variant + 5 for a bordered cell; percentages last.
const BOLD: usize = 1;
const ITALIC: usize = 2;
const HEADING: usize = 4;
const BORDERED: usize = 5;
const PERCENT: usize = 10;

/// A value in a cell.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Value {
    Text(String),
    Number(f64),
    /// A percentage, stored as its fraction (`12.5%` is 0.125) and shown as a percentage.
    Percent(f64),
}

/// Read `s` as a number when it is one and nothing else: digits with an optional sign, decimal
/// point and thousands commas, an accounting negative in parentheses, or a trailing `%`. Anything
/// that would change by becoming a number stays text: a leading zero (`0771234567`), more than
/// [`MAX_DIGITS`] digits, an exponent, dates, codes, currency words.
pub(crate) fn value(s: &str) -> Value {
    let text = || Value::Text(s.to_string());
    let t = s.trim();
    if t.is_empty() || t.len() > 40 {
        return text();
    }
    let (t, negative_parens) = match t.strip_prefix('(').and_then(|r| r.strip_suffix(')')) {
        Some(inner) => (inner.trim(), true),
        None => (t, false),
    };
    let (t, percent) = match t.strip_suffix('%') {
        Some(rest) => (rest.trim_end(), true),
        None => (t, false),
    };
    let (t, negative_sign) = match t.strip_prefix('-').or_else(|| t.strip_prefix('\u{2212}')) {
        Some(rest) => (rest, true),
        None => (t.strip_prefix('+').unwrap_or(t), false),
    };
    if negative_parens && negative_sign {
        return text();
    }
    let (int, frac) = match t.split_once('.') {
        Some((i, f)) => (i, Some(f)),
        None => (t, None),
    };
    if int.is_empty() || !int.starts_with(|c: char| c.is_ascii_digit()) {
        return text();
    }
    // Thousands commas only in their places: 1,234,567 but not 12,34 or 1,2345.
    let groups: Vec<&str> = int.split(',').collect();
    let grouped = groups.len() > 1;
    if grouped {
        let first = groups.first().map_or(0, |g| g.len());
        if !(1..=3).contains(&first) || groups.iter().skip(1).any(|g| g.len() != 3) {
            return text();
        }
    }
    let digits: String = groups.concat();
    if !digits.chars().all(|c| c.is_ascii_digit()) {
        return text();
    }
    if let Some(f) = frac
        && (f.is_empty() || !f.chars().all(|c| c.is_ascii_digit()))
    {
        return text();
    }
    // A leading zero is part of an identifier (phone numbers, codes), except in 0 and 0.5.
    if digits.len() > 1 && digits.starts_with('0') {
        return text();
    }
    let significant = digits.trim_start_matches('0').len() + frac.map_or(0, str::len);
    if significant > MAX_DIGITS {
        return text();
    }
    let literal = match frac {
        Some(f) => format!("{digits}.{f}"),
        None => digits,
    };
    let Ok(mut n) = literal.parse::<f64>() else { return text() };
    if !n.is_finite() {
        return text();
    }
    if negative_parens || negative_sign {
        n = -n;
    }
    if percent { Value::Percent(n / 100.0) } else { Value::Number(n) }
}

/// The column letters of 0-based column `col` (0 is A, 26 is AA).
pub(crate) fn column_name(col: usize) -> String {
    let mut n = col.saturating_add(1);
    let mut out = Vec::new();
    while n > 0 {
        let rem = (n - 1) % 26;
        out.push(b'A' + rem as u8);
        n = (n - 1) / 26;
    }
    out.reverse();
    String::from_utf8(out).unwrap_or_default()
}

/// `s` cut to Excel's cell size, at a character boundary.
fn cell_text(s: &str) -> &str {
    match s.char_indices().nth(MAX_CELL_CHARS) {
        Some((i, _)) => s.get(..i).unwrap_or(s),
        None => s,
    }
}

/// A sheet being written: its rows (XML), merged ranges and the widths its tables ask for.
#[derive(Default)]
struct Sheet {
    rows: String,
    /// Rows used so far (the next row is `used + 1`).
    used: usize,
    merges: Vec<String>,
    widths: Vec<usize>,
}

impl Sheet {
    fn full(&self) -> bool {
        self.used >= MAX_ROWS
    }

    /// Start a row; `None` when the sheet is full.
    fn next_row(&mut self) -> Option<usize> {
        if self.full() {
            return None;
        }
        self.used += 1;
        Some(self.used)
    }

    fn cell(out: &mut String, row: usize, col: usize, v: &Value, style: usize) {
        let r = format!("{}{row}", column_name(col));
        match v {
            Value::Text(s) if s.is_empty() => out.push_str(&format!("<c r=\"{r}\" s=\"{style}\"/>")),
            Value::Text(s) => {
                out.push_str(&format!("<c r=\"{r}\" s=\"{style}\" t=\"inlineStr\"><is><t xml:space=\"preserve\">{}</t></is></c>", esc(cell_text(s))))
            }
            Value::Number(n) => out.push_str(&format!("<c r=\"{r}\" s=\"{style}\"><v>{n}</v></c>")),
            Value::Percent(n) => {
                let style = if style >= BORDERED { PERCENT + 1 } else { PERCENT };
                out.push_str(&format!("<c r=\"{r}\" s=\"{style}\"><v>{n}</v></c>"));
            }
        }
    }

    /// A paragraph: its text in column A, one row.
    fn paragraph(&mut self, text: &str, style: usize) {
        let text = text.trim();
        if text.is_empty() {
            return;
        }
        let Some(row) = self.next_row() else { return };
        let mut xml = format!("<row r=\"{row}\">");
        Self::cell(&mut xml, row, 0, &value(text), style);
        xml.push_str("</row>");
        self.rows.push_str(&xml);
    }

    fn blank(&mut self) {
        // Rows that hold nothing are simply skipped: the next row number leaves the gap.
        if self.used > 0 && !self.full() {
            self.used += 1;
        }
    }

    fn table(&mut self, t: &Table) {
        let bordered = t.ruled.is_some();
        for cells in &t.rows {
            let Some(row) = self.next_row() else { return };
            let mut xml = format!("<row r=\"{row}\">");
            let mut col = 0usize;
            for c in cells {
                let span = c.span.max(1);
                let font = match (c.bold, c.italic) {
                    (true, true) => 3,
                    (true, false) => BOLD,
                    (false, true) => ITALIC,
                    (false, false) => 0,
                };
                let style = font + if bordered { BORDERED } else { 0 };
                let text = c.text.trim();
                Self::cell(&mut xml, row, col, &value(text), style);
                if span > 1 {
                    let last = col.saturating_add(span - 1);
                    // Bordered spans keep their border on every merged cell.
                    for extra in col + 1..=last {
                        Self::cell(&mut xml, row, extra, &Value::Text(String::new()), style);
                    }
                    self.merges.push(format!("{}{row}:{}{row}", column_name(col), column_name(last)));
                } else {
                    let want = text.lines().map(|l| l.chars().count()).max().unwrap_or(0).saturating_add(2).clamp(MIN_WIDTH, MAX_WIDTH);
                    if self.widths.len() <= col {
                        self.widths.resize(col + 1, MIN_WIDTH);
                    }
                    if let Some(w) = self.widths.get_mut(col) {
                        *w = (*w).max(want);
                    }
                }
                col = col.saturating_add(span);
            }
            xml.push_str("</row>");
            self.rows.push_str(&xml);
        }
    }

    fn xml(&self) -> String {
        let mut s = String::from(
            "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<worksheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\">",
        );
        if !self.widths.is_empty() {
            s.push_str("<cols>");
            for (i, w) in self.widths.iter().enumerate() {
                s.push_str(&format!("<col min=\"{n}\" max=\"{n}\" width=\"{w}\" customWidth=\"1\"/>", n = i + 1));
            }
            s.push_str("</cols>");
        }
        s.push_str("<sheetData>");
        s.push_str(&self.rows);
        s.push_str("</sheetData>");
        if !self.merges.is_empty() {
            s.push_str(&format!("<mergeCells count=\"{}\">", self.merges.len()));
            for m in &self.merges {
                s.push_str(&format!("<mergeCell ref=\"{m}\"/>"));
            }
            s.push_str("</mergeCells>");
        }
        s.push_str("</worksheet>");
        s
    }
}

fn page_sheet(page: &Page) -> Sheet {
    let mut sheet = Sheet::default();
    let mut last_was_table = false;
    for it in items(std::slice::from_ref(page)) {
        if sheet.full() {
            break;
        }
        match it {
            Item::Para(runs, level) => {
                if last_was_table {
                    sheet.blank();
                }
                let text: String = runs.iter().map(|r| r.text.as_str()).collect();
                let style = match (level, runs.first().map(|r| (r.block.bold, r.block.italic))) {
                    (1 | 2, _) => HEADING,
                    (_, Some((true, true))) => 3,
                    (_, Some((true, false))) => BOLD,
                    (_, Some((false, true))) => ITALIC,
                    _ => 0,
                };
                sheet.paragraph(&text, style);
                last_was_table = false;
            }
            Item::Table(t) => {
                sheet.blank();
                sheet.table(&t);
                last_was_table = true;
            }
            Item::Img(_) | Item::PageBreak => {}
        }
    }
    sheet
}

/// The sheet name for page `index` (0-based): "Page 1", "Page 2", …, within Excel's limits.
fn sheet_name(index: usize) -> String {
    let name = format!("Page {}", index.saturating_add(1));
    name.chars().take(MAX_SHEET_NAME).collect()
}

const STYLES: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<styleSheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\">\
<fonts count=\"5\">\
<font><sz val=\"11\"/><name val=\"Calibri\"/><family val=\"2\"/></font>\
<font><b/><sz val=\"11\"/><name val=\"Calibri\"/><family val=\"2\"/></font>\
<font><i/><sz val=\"11\"/><name val=\"Calibri\"/><family val=\"2\"/></font>\
<font><b/><i/><sz val=\"11\"/><name val=\"Calibri\"/><family val=\"2\"/></font>\
<font><b/><sz val=\"14\"/><name val=\"Calibri\"/><family val=\"2\"/></font>\
</fonts>\
<fills count=\"2\"><fill><patternFill patternType=\"none\"/></fill><fill><patternFill patternType=\"gray125\"/></fill></fills>\
<borders count=\"2\"><border><left/><right/><top/><bottom/><diagonal/></border>\
<border><left style=\"thin\"><color auto=\"1\"/></left><right style=\"thin\"><color auto=\"1\"/></right><top style=\"thin\"><color auto=\"1\"/></top><bottom style=\"thin\"><color auto=\"1\"/></bottom><diagonal/></border></borders>\
<cellStyleXfs count=\"1\"><xf numFmtId=\"0\" fontId=\"0\" fillId=\"0\" borderId=\"0\"/></cellStyleXfs>\
<cellXfs count=\"12\">\
<xf numFmtId=\"0\" fontId=\"0\" fillId=\"0\" borderId=\"0\" xfId=\"0\"/>\
<xf numFmtId=\"0\" fontId=\"1\" fillId=\"0\" borderId=\"0\" xfId=\"0\" applyFont=\"1\"/>\
<xf numFmtId=\"0\" fontId=\"2\" fillId=\"0\" borderId=\"0\" xfId=\"0\" applyFont=\"1\"/>\
<xf numFmtId=\"0\" fontId=\"3\" fillId=\"0\" borderId=\"0\" xfId=\"0\" applyFont=\"1\"/>\
<xf numFmtId=\"0\" fontId=\"4\" fillId=\"0\" borderId=\"0\" xfId=\"0\" applyFont=\"1\"/>\
<xf numFmtId=\"0\" fontId=\"0\" fillId=\"0\" borderId=\"1\" xfId=\"0\" applyBorder=\"1\"/>\
<xf numFmtId=\"0\" fontId=\"1\" fillId=\"0\" borderId=\"1\" xfId=\"0\" applyFont=\"1\" applyBorder=\"1\"/>\
<xf numFmtId=\"0\" fontId=\"2\" fillId=\"0\" borderId=\"1\" xfId=\"0\" applyFont=\"1\" applyBorder=\"1\"/>\
<xf numFmtId=\"0\" fontId=\"3\" fillId=\"0\" borderId=\"1\" xfId=\"0\" applyFont=\"1\" applyBorder=\"1\"/>\
<xf numFmtId=\"0\" fontId=\"4\" fillId=\"0\" borderId=\"1\" xfId=\"0\" applyFont=\"1\" applyBorder=\"1\"/>\
<xf numFmtId=\"10\" fontId=\"0\" fillId=\"0\" borderId=\"0\" xfId=\"0\" applyNumberFormat=\"1\"/>\
<xf numFmtId=\"10\" fontId=\"0\" fillId=\"0\" borderId=\"1\" xfId=\"0\" applyNumberFormat=\"1\" applyBorder=\"1\"/>\
</cellXfs>\
<cellStyles count=\"1\"><cellStyle name=\"Normal\" xfId=\"0\" builtinId=\"0\"/></cellStyles>\
</styleSheet>";

/// The document as an Excel workbook: one sheet per page (at least one sheet, which Excel needs).
pub fn xlsx(pages: &[Page], title: &str) -> Vec<u8> {
    let mut sheets: Vec<String> = pages.iter().map(|p| page_sheet(p).xml()).collect();
    if sheets.is_empty() {
        sheets.push(Sheet::default().xml());
    }
    let mut types = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\">\
<Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/>\
<Default Extension=\"xml\" ContentType=\"application/xml\"/>\
<Override PartName=\"/xl/workbook.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml\"/>\
<Override PartName=\"/xl/styles.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml\"/>\
<Override PartName=\"/docProps/core.xml\" ContentType=\"application/vnd.openxmlformats-package.core-properties+xml\"/>",
    );
    let mut workbook = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<workbook xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\"><sheets>",
    );
    let mut wb_rels = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">\
<Relationship Id=\"rIdStyles\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles\" Target=\"styles.xml\"/>",
    );
    for i in 0..sheets.len() {
        let n = i + 1;
        types.push_str(&format!(
            "<Override PartName=\"/xl/worksheets/sheet{n}.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml\"/>"
        ));
        workbook.push_str(&format!("<sheet name=\"{}\" sheetId=\"{n}\" r:id=\"rIdSheet{n}\"/>", esc(&sheet_name(i))));
        wb_rels.push_str(&format!(
            "<Relationship Id=\"rIdSheet{n}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet\" Target=\"worksheets/sheet{n}.xml\"/>"
        ));
    }
    types.push_str("</Types>");
    workbook.push_str("</sheets></workbook>");
    wb_rels.push_str("</Relationships>");
    let rels = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">\
<Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"xl/workbook.xml\"/>\
<Relationship Id=\"rId2\" Type=\"http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties\" Target=\"docProps/core.xml\"/></Relationships>";
    let core = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<cp:coreProperties xmlns:cp=\"http://schemas.openxmlformats.org/package/2006/metadata/core-properties\" xmlns:dc=\"http://purl.org/dc/elements/1.1/\"><dc:title>{}</dc:title></cp:coreProperties>",
        esc(title)
    );
    let mut z = Zip::default();
    z.add("[Content_Types].xml", types.as_bytes(), true);
    z.add("_rels/.rels", rels.as_bytes(), true);
    z.add("docProps/core.xml", core.as_bytes(), true);
    z.add("xl/workbook.xml", workbook.as_bytes(), true);
    z.add("xl/_rels/workbook.xml.rels", wb_rels.as_bytes(), true);
    z.add("xl/styles.xml", STYLES.as_bytes(), true);
    for (i, sheet) in sheets.iter().enumerate() {
        z.add(&format!("xl/worksheets/sheet{}.xml", i + 1), sheet.as_bytes(), true);
    }
    z.finish()
}
