//! Read-only SD application catalog scanner.

use std::path::{Path, PathBuf};

use super::manifest::{is_fat83_safe_directory_name, read_manifest, LuaAppEntry};

pub const LUA_APPS_DIRECTORY: &str = "/sdcard/RUSTMIX/APPS";
pub const LUA_APP_CATALOG_LIMIT: usize = 32;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LuaAppCatalog {
    pub root: PathBuf,
    pub entries: Vec<LuaAppEntry>,
    pub raw_entries: usize,
    pub rejected_entries: usize,
    pub warning: Option<String>,
}

impl LuaAppCatalog {
    #[must_use]
    pub fn unavailable(root: impl Into<PathBuf>, warning: impl Into<String>) -> Self {
        Self {
            root: root.into(),
            entries: Vec::new(),
            raw_entries: 0,
            rejected_entries: 0,
            warning: Some(warning.into()),
        }
    }

    pub fn scan(root: impl Into<PathBuf>, mounted: bool) -> Self {
        let root = root.into();
        if !mounted {
            return Self::unavailable(root, "SD card is unavailable");
        }
        match scan_directory(&root) {
            Ok(catalog) => catalog,
            Err(error) => Self::unavailable(root, error),
        }
    }

    #[must_use]
    pub const fn is_available(&self) -> bool {
        self.warning.is_none()
    }
}

fn scan_directory(root: &Path) -> Result<LuaAppCatalog, String> {
    let mut catalog = LuaAppCatalog {
        root: root.to_path_buf(),
        ..LuaAppCatalog::default()
    };
    let directory =
        std::fs::read_dir(root).map_err(|error| format!("{}: {error}", root.display()))?;
    for item in directory {
        let item = match item {
            Ok(item) => item,
            Err(_) => {
                catalog.rejected_entries = catalog.rejected_entries.saturating_add(1);
                continue;
            }
        };
        catalog.raw_entries = catalog.raw_entries.saturating_add(1);
        if catalog.entries.len() >= LUA_APP_CATALOG_LIMIT {
            catalog.rejected_entries = catalog.rejected_entries.saturating_add(1);
            continue;
        }
        let path = item.path();
        let directory_name = item.file_name().to_string_lossy().into_owned();
        if !path.is_dir() || !is_fat83_safe_directory_name(&directory_name) {
            catalog.rejected_entries = catalog.rejected_entries.saturating_add(1);
            continue;
        }
        let manifest = match read_manifest(&path) {
            Ok(manifest) => manifest,
            Err(_) => {
                catalog.rejected_entries = catalog.rejected_entries.saturating_add(1);
                continue;
            }
        };
        if !path.join(&manifest.entry).is_file() {
            catalog.rejected_entries = catalog.rejected_entries.saturating_add(1);
            continue;
        }
        catalog.entries.push(LuaAppEntry {
            directory_name,
            directory: path,
            manifest,
        });
    }
    catalog
        .entries
        .sort_by(|left, right| left.manifest.name.cmp(&right.manifest.name));
    Ok(catalog)
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::LuaAppCatalog;

    fn temp_directory() -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("rustmix-lua-catalog-{nonce}"))
    }

    #[test]
    fn scans_only_fat83_manifest_apps() {
        let root = temp_directory();
        let app = root.join("HGRID");
        std::fs::create_dir_all(&app).unwrap();
        std::fs::write(
            app.join("APP.TOM"),
            "id=\"hello_grid\"\nname=\"Hello Grid\"\nkind=\"game\"\nentry=\"MAIN.LUA\"\n",
        )
        .unwrap();
        std::fs::write(app.join("MAIN.LUA"), "ui.clear()\n").unwrap();
        std::fs::create_dir_all(root.join("not-fat83-long")).unwrap();

        let catalog = LuaAppCatalog::scan(&root, true);
        assert_eq!(catalog.entries.len(), 1);
        assert_eq!(catalog.entries[0].manifest.id, "hello_grid");
        assert_eq!(catalog.rejected_entries, 1);
        std::fs::remove_dir_all(root).unwrap();
    }
}
