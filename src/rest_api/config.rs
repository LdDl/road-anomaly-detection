use crate::app::ZoneSettings;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
use toml_edit::{Array, ArrayOfTables, DocumentMut, Item, Table, Value, value};
use uuid::Uuid;

pub fn update_document(contents: &str, zones: &Option<Vec<ZoneSettings>>) -> Result<String, String> {
    let mut document = contents.parse::<DocumentMut>().map_err(|err| err.to_string())?;
    match zones {
        None => { document.remove("zones"); },
        Some(zones) if zones.is_empty() => {
            document["zones"] = value(Array::new());
        },
        Some(zones) => {
            let mut tables = ArrayOfTables::new();
            for zone in zones {
                let mut table = Table::new();
                table["id"] = value(zone.id.clone());
                let geometry: Array = zone.geometry.iter().map(|point| {
                    Value::Array(point.iter().map(|coordinate| *coordinate as i64).collect())
                }).collect();
                table["geometry"] = value(geometry);
                if let Some(color) = zone.color_rgb {
                    table["color_rgb"] = value(color.iter().map(|component| *component as i64).collect::<Array>());
                }
                tables.push(table);
            }
            document["zones"] = Item::ArrayOfTables(tables);
        }
    }
    Ok(document.to_string())
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
