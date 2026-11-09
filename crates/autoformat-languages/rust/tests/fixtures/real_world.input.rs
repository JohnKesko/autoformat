use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Errors that can happen while loading an inventory file.
#[derive(Debug)]
pub enum InventoryError {
    Io { path: PathBuf, source: std::io::Error },
    Parse { line: usize, message: String },
}

impl std::fmt::Display for InventoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InventoryError::Io { path, source } => write!(f, "cannot read {}: {}", path.display(), source),
            InventoryError::Parse { line, message } => write!(f, "line {}: {}", line, message),
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct Inventory {
    items: BTreeMap<String, u32>,
}

impl Inventory {
    pub fn load(path: &Path) -> Result<Inventory, InventoryError> {
        let text = fs::read_to_string(path).map_err(|source| InventoryError::Io { path: path.to_path_buf(), source })?;
        let mut inventory = Inventory::default();
        for (index, line) in text.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let (name, count) = match line.split_once(':') {
                Some(parts) => parts,
                None => return Err(InventoryError::Parse { line: index + 1, message: format!("expected `name: count`, found `{}`", line) }),
            };
            let count: u32 = count.trim().parse().map_err(|_| InventoryError::Parse {
                line: index + 1,
                message: "count is not a number".to_string(),
            })?;
            *inventory.items.entry(name.trim().to_string()).or_insert(0) += count;
        }
        Ok(inventory)
    }

    pub fn total(&self) -> u64 {
        self.items.values().map(|&count| count as u64).sum()
    }

    pub fn most_common(&self, limit: usize) -> Vec<(&str, u32)> {
        let mut entries: Vec<(&str, u32)> = self.items.iter().map(|(name, &count)| (name.as_str(), count)).collect();
        entries.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
        entries.truncate(limit);
        entries
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn totals_are_added_up() {
        let mut inventory = Inventory::default();
        inventory.items.insert("apples".to_string(), 3);
        inventory.items.insert("pears".to_string(), 4);
        assert_eq!(inventory.total(), 7);
        assert_eq!(inventory.most_common(1), vec![("pears", 4)]);
    }
}
