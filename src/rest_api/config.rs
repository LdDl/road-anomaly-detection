use crate::app::ZoneSettings;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
use toml_edit::{Array, ArrayOfTables, Decor, DocumentMut, Item, Table, Value, value};
use uuid::Uuid;

pub fn update_document(contents: &str, zones: &Option<Vec<ZoneSettings>>) -> Result<String, String> {
    let mut document = contents.parse::<DocumentMut>().map_err(|err| err.to_string())?;
    match zones {
        None => { document.remove("zones"); },
        Some(zones) if zones.is_empty() => {
            merge_field(document.as_table_mut(), "zones", Value::Array(Array::new()));
        },
        Some(zones) => {
            let current = document.get("zones").and_then(Item::as_array_of_tables).cloned().unwrap_or_default();
            let positions: Vec<_> = current.iter().filter_map(Table::position).collect();
            let indent = current.iter().next().map(key_indent).unwrap_or_else(|| "    ".to_string());
            let header_indent = current.iter().last().map(|table| indentation(table.decor())).unwrap_or_default();
            let mut previous: Vec<_> = current.into_iter().map(Some).collect();
            let mut tables = ArrayOfTables::new();
            for (index, zone) in zones.iter().enumerate() {
                // Match IDs first so deleting a zone does not move its comments to another zone.
                let matching = previous.iter().position(|table| {
                    table.as_ref().and_then(|table| table.get("id")).and_then(Item::as_str) == Some(zone.id.as_str())
                });
                // A renamed zone can reuse its slot unless that ID is still in the requested list.
                let matching = matching.or_else(|| {
                    let table = previous.get(index)?.as_ref()?;
                    let id = table.get("id").and_then(Item::as_str)?;
                    if zones.iter().any(|zone| zone.id == id) { None } else { Some(index) }
                });
                let mut table = matching.and_then(|index| previous[index].take()).unwrap_or_else(|| {
                    let mut table = Table::new();
                    table.decor_mut().set_prefix(format!("\n{}", header_indent));
                    table
                });
                if let Some(position) = positions.get(index).or_else(|| positions.last()) {
                    table.set_position(*position);
                }
                for key in ["id", "geometry", "color_rgb"] {
                    if !table.contains_key(key) {
                        table[key] = value("");
                        table.key_mut(key).unwrap().leaf_decor_mut().set_prefix(indent.clone());
                    }
                }
                merge_field(&mut table, "id", Value::from(zone.id.clone()));
                let geometry: Array = zone.geometry.iter().map(|point| {
                    Value::Array(point.iter().map(|coordinate| *coordinate as i64).collect())
                }).collect();
                merge_field(&mut table, "geometry", Value::Array(geometry));
                if let Some(color) = zone.color_rgb {
                    merge_field(&mut table, "color_rgb", Value::Array(color.iter().map(|component| *component as i64).collect()));
                } else {
                    table.remove("color_rgb");
                }
                tables.push(table);
            }
            document["zones"] = Item::ArrayOfTables(tables);
        }
    }
    Ok(document.to_string())
}

fn merge_field(table: &mut Table, key: &str, value: Value) {
    match table.get_mut(key).and_then(Item::as_value_mut) {
        Some(current) => merge_value(current, value),
        None => { table[key] = Item::Value(value); },
    }
}

fn merge_value(current: &mut Value, mut value: Value) {
    match (&mut *current, &value) {
        (Value::String(old), Value::String(new)) if old.value() == new.value() => return,
        (Value::Integer(old), Value::Integer(new)) if old.value() == new.value() => return,
        (Value::Array(old), Value::Array(new)) if old.len() == new.len() => {
            // Update coordinates in place, including comments and spacing inside multiline arrays.
            for (old, new) in old.iter_mut().zip(new.iter()) {
                merge_value(old, new.clone());
            }
            return;
        },
        _ => {}
    }
    *value.decor_mut() = current.decor().clone();
    *current = value;
}

fn key_indent(table: &Table) -> String {
    table.iter().next().and_then(|(key, _)| table.key(key)).map(|key| indentation(key.leaf_decor())).unwrap_or_default()
}

fn indentation(decor: &Decor) -> String {
    decor.prefix().and_then(|prefix| prefix.as_str()).and_then(|prefix| prefix.rsplit('\n').next())
        .filter(|prefix| prefix.chars().all(char::is_whitespace)).unwrap_or("").to_string()
}

pub fn save_document(path: &Path, contents: &str) -> io::Result<String> {
    let suffix = Uuid::new_v4().to_string();
    let filename = path.file_name().ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Invalid configuration path"))?.to_string_lossy();
    let temporary = path.with_file_name(format!(".{}.{}.tmp", filename, suffix));
    let backup = path.with_file_name(format!("{}.{}.bak", filename, suffix));
    let result = (|| {
        let mut file = OpenOptions::new().write(true).create_new(true).open(&temporary)?;
        file.set_permissions(fs::metadata(path)?.permissions())?;
        file.write_all(contents.as_bytes())?;
        file.sync_all()?;
        fs::copy(path, &backup)?;
        // Keep the old configuration intact if writing the replacement fails.
        fs::rename(&temporary, path)?;
        Ok(backup.file_name().unwrap().to_string_lossy().to_string())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}
