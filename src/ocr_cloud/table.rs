//! Map the provider's declared rows, columns and inclusive spans faithfully.
//! A missing header is valid; no row is classified or removed by its contents.
use super::*;
fn notes(data: &Value, id: &Value, key: &str) -> String {
    data.get("tableHeadTail")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|n| n.get("tableId") == Some(id))
        .filter_map(|n| n.get(key).and_then(Value::as_array))
        .flatten()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>()
        .join("\n")
}
pub(super) fn parse(data: &Value) -> Result<Vec<Table>, String> {
    let entries = data
        .get("prism_tablesInfo")
        .and_then(Value::as_array)
        .ok_or("阿里云未返回 prism_tablesInfo。")?;
    let mut tables = Vec::new();
    for entry in entries {
        let mut table = Table {
            header: notes(data, &entry["tableId"], "head"),
            footer: notes(data, &entry["tableId"], "tail"),
            ..Table::default()
        };
        for cell in entry
            .get("cellInfos")
            .and_then(Value::as_array)
            .ok_or("阿里云表格缺少 cellInfos。")?
        {
            let row = number(cell, "ysc")?;
            let col = number(cell, "xsc")?;
            let end_row = number(cell, "yec")?;
            let end_col = number(cell, "xec")?;
            if end_row < row || end_col < col {
                return Err("表格范围无效。".into());
            }
            add_cell(
                &mut table,
                row,
                col,
                end_row.checked_add(1).ok_or("表格行溢出。")?,
                end_col.checked_add(1).ok_or("表格列溢出。")?,
                cell.get("word").and_then(Value::as_str).unwrap_or(""),
            )?;
        }
        // Explicit dimensions preserve empty edge rows/columns in blank forms.
        for (name, actual) in [
            ("yCellSize", &mut table.rows),
            ("xCellSize", &mut table.cols),
        ] {
            if entry.get(name).is_some() {
                let declared = number(entry, name)?;
                if declared < *actual {
                    return Err("接口声明的行列数小于单元格范围。".into());
                }
                *actual = declared;
            }
        }
        table.grid()?;
        tables.push(table);
    }
    if tables.is_empty() {
        return Err("阿里云未检测到表格，请截取包含完整行列的区域。".into());
    }
    Ok(tables)
}
