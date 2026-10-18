//! Excel XML Spreadsheet clipboard representation using quick-xml's writer.
use super::*;
use quick_xml::{
    events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event},
    Writer,
};
use std::io;
impl Table {
    pub fn spreadsheet_xml(&self) -> Result<Vec<u8>, String> {
        self.grid()?;
        write(self).map_err(|_| "无法生成 Excel 表格。".into())
    }
}
fn start(w: &mut Writer<Vec<u8>>, name: &str, attrs: &[(&str, String)]) -> io::Result<()> {
    let mut element = BytesStart::new(name);
    for (key, value) in attrs {
        element.push_attribute((*key, value.as_str()));
    }
    w.write_event(Event::Start(element))
}
fn end(w: &mut Writer<Vec<u8>>, name: &str) -> io::Result<()> {
    w.write_event(Event::End(BytesEnd::new(name)))
}
fn write(table: &Table) -> io::Result<Vec<u8>> {
    let mut w = Writer::new(vec![0xef, 0xbb, 0xbf]);
    w.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))?;
    start(
        &mut w,
        "Workbook",
        &[
            (
                "xmlns",
                "urn:schemas-microsoft-com:office:spreadsheet".into(),
            ),
            (
                "xmlns:ss",
                "urn:schemas-microsoft-com:office:spreadsheet".into(),
            ),
        ],
    )?;
    start(&mut w, "Styles", &[])?;
    start(
        &mut w,
        "Style",
        &[("ss:ID", "Default".into()), ("ss:Name", "Normal".into())],
    )?;
    start(
        &mut w,
        "Alignment",
        &[("ss:Vertical", "Top".into()), ("ss:WrapText", "1".into())],
    )?;
    end(&mut w, "Alignment")?;
    start(&mut w, "Borders", &[])?;
    for edge in ["Left", "Top", "Right", "Bottom"] {
        start(
            &mut w,
            "Border",
            &[
                ("ss:Position", edge.into()),
                ("ss:LineStyle", "Continuous".into()),
                ("ss:Weight", "1".into()),
            ],
        )?;
        end(&mut w, "Border")?;
    }
    end(&mut w, "Borders")?;
    end(&mut w, "Style")?;
    for (id, format) in std::iter::once(("text".into(), "@".into()))
        .chain(std::iter::once((
            "number".into(),
            "#,##0.################".into(),
        )))
        .chain((0..=8).map(|n| {
            (
                format!("pct{n}"),
                if n == 0 {
                    "0%".into()
                } else {
                    format!("0.{}%", "0".repeat(n))
                },
            )
        }))
    {
        start(
            &mut w,
            "Style",
            &[("ss:ID", id), ("ss:Parent", "Default".into())],
        )?;
        start(&mut w, "NumberFormat", &[("ss:Format", format)])?;
        end(&mut w, "NumberFormat")?;
        end(&mut w, "Style")?;
    }
    end(&mut w, "Styles")?;
    start(&mut w, "Worksheet", &[("ss:Name", "Table".into())])?;
    start(
        &mut w,
        "Table",
        &[
            ("ss:ExpandedRowCount", table.rows.to_string()),
            ("ss:ExpandedColumnCount", table.cols.to_string()),
        ],
    )?;
    for row in 0..table.rows {
        start(&mut w, "Row", &[("ss:Index", (row + 1).to_string())])?;
        for col in 0..table.cols {
            let Some(cell) = table.at(row, col) else {
                continue;
            };
            let numeric = !needs_excel_text(&cell.text);
            let percent = numeric && cell.text.trim().ends_with('%');
            let style = if percent {
                let decimals = cell
                    .text
                    .trim()
                    .trim_end_matches('%')
                    .split('.')
                    .nth(1)
                    .map_or(0, str::len)
                    .min(8);
                format!("pct{decimals}")
            } else if numeric {
                "number".into()
            } else {
                "text".into()
            };
            let mut attrs = vec![("ss:Index", (col + 1).to_string()), ("ss:StyleID", style)];
            if cell.colspan > 1 {
                attrs.push(("ss:MergeAcross", (cell.colspan - 1).to_string()));
            }
            if cell.rowspan > 1 {
                attrs.push(("ss:MergeDown", (cell.rowspan - 1).to_string()));
            }
            start(&mut w, "Cell", &attrs)?;
            start(
                &mut w,
                "Data",
                &[("ss:Type", if numeric { "Number" } else { "String" }.into())],
            )?;
            let value = if numeric {
                let normalized = cell.text.trim().replace([',', '，'], "");
                if percent {
                    (normalized.trim_end_matches('%').parse::<f64>().unwrap() / 100.).to_string()
                } else {
                    normalized
                }
            } else {
                cell.text.replace("\r\n", "\n").replace('\r', "\n")
            };
            let value = quick_xml::escape::escape(&value)
                .replace('\n', "&#10;")
                .replace('\t', "&#9;");
            w.write_event(Event::Text(BytesText::from_escaped(value)))?;
            end(&mut w, "Data")?;
            end(&mut w, "Cell")?;
        }
        end(&mut w, "Row")?;
    }
    end(&mut w, "Table")?;
    end(&mut w, "Worksheet")?;
    end(&mut w, "Workbook")?;
    Ok(w.into_inner())
}
