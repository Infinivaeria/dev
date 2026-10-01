//! Game-style inventory attached to every grid: named items with a
//! quantity and free-form JSON metadata. It is stored with the grid it
//! belongs to, so each nested grid (level, room, chest, ...) has its own.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Longest accepted item name, in characters.
pub const MAX_NAME_CHARS: usize = 64;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct InventoryItem {
    pub count: i64,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub meta: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GridInventory {
    items: BTreeMap<String, InventoryItem>,
}

/// Trims an item name and checks it is usable.
pub fn normalize_name(name: &str) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("inventory item names can't be empty".to_owned());
    }
    if name.chars().count() > MAX_NAME_CHARS {
        return Err(format!(
            "inventory item names are limited to {MAX_NAME_CHARS} characters"
        ));
    }
    if name.chars().any(char::is_control) {
        return Err("inventory item names can't contain control characters".to_owned());
    }
    Ok(name.to_owned())
}

impl GridInventory {
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Sum of all item counts.
    pub fn total(&self) -> i64 {
        self.items
            .values()
            .fold(0i64, |sum, item| sum.saturating_add(item.count))
    }

    pub fn count(&self, name: &str) -> i64 {
        self.items.get(name.trim()).map_or(0, |item| item.count)
    }

    #[cfg_attr(not(feature = "scripting"), allow(dead_code))]
    pub fn has(&self, name: &str, amount: i64) -> bool {
        self.items.contains_key(name.trim()) && self.count(name) >= amount
    }

    pub fn get(&self, name: &str) -> Option<&InventoryItem> {
        self.items.get(name.trim())
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &InventoryItem)> {
        self.items.iter().map(|(name, item)| (name.as_str(), item))
    }

    /// Adds `amount` (> 0) of `name`; returns the new count.
    pub fn add(&mut self, name: &str, amount: i64) -> Result<i64, String> {
        let name = normalize_name(name)?;
        if amount <= 0 {
            return Err(format!("can only add a positive amount, got {amount}"));
        }
        let item = self.items.entry(name).or_default();
        item.count = item.count.saturating_add(amount);
        Ok(item.count)
    }

    /// Removes `amount` (> 0) of `name`; fails without changing anything if
    /// there isn't enough. An item whose count reaches zero is dropped.
    /// Returns the remaining count.
    pub fn remove(&mut self, name: &str, amount: i64) -> Result<i64, String> {
        let name = normalize_name(name)?;
        if amount <= 0 {
            return Err(format!("can only remove a positive amount, got {amount}"));
        }
        let have = self.count(&name);
        if have < amount {
            return Err(format!("not enough {name}: have {have}, need {amount}"));
        }
        let left = have - amount;
        if left == 0 {
            self.items.remove(&name);
        } else if let Some(item) = self.items.get_mut(&name) {
            item.count = left;
        }
        Ok(left)
    }

    /// Sets the count of `name`; zero or less removes the item.
    pub fn set(&mut self, name: &str, count: i64) -> Result<(), String> {
        let name = normalize_name(name)?;
        if count <= 0 {
            self.items.remove(&name);
        } else {
            self.items.entry(name).or_default().count = count;
        }
        Ok(())
    }

    /// Removes an item entirely; returns whether it existed.
    pub fn delete(&mut self, name: &str) -> bool {
        self.items.remove(name.trim()).is_some()
    }

    /// Renames an item, merging counts and metadata into an existing one.
    pub fn rename(&mut self, from: &str, to: &str) -> Result<(), String> {
        let to = normalize_name(to)?;
        let Some(item) = self.items.remove(from.trim()) else {
            return Err(format!("no inventory item named {:?}", from.trim()));
        };
        let target = self.items.entry(to).or_default();
        target.count = target.count.saturating_add(item.count);
        target.meta.extend(item.meta);
        Ok(())
    }

    /// Sets (or with `Value::Null`, clears) one metadata key of an existing item.
    pub fn set_meta(&mut self, name: &str, key: &str, value: Value) -> Result<(), String> {
        let item = self
            .items
            .get_mut(name.trim())
            .ok_or_else(|| format!("no inventory item named {:?}", name.trim()))?;
        if value.is_null() {
            item.meta.remove(key);
        } else {
            item.meta.insert(key.to_owned(), value);
        }
        Ok(())
    }

    /// Moves `amount` of `name` into `other`, all or nothing.
    #[cfg_attr(not(feature = "scripting"), allow(dead_code))]
    pub fn transfer(
        &mut self,
        other: &mut GridInventory,
        name: &str,
        amount: i64,
    ) -> Result<(), String> {
        let meta = self.get(name).map(|item| item.meta.clone());
        self.remove(name, amount)?;
        other.add(name, amount)?;
        if let (Some(meta), Some(item)) = (meta, other.items.get_mut(name.trim())) {
            for (key, value) in meta {
                item.meta.entry(key).or_insert(value);
            }
        }
        Ok(())
    }

    pub fn clear(&mut self) {
        self.items.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn adds_removes_and_drops_empty_items() {
        let mut inv = GridInventory::default();
        assert_eq!(inv.add(" potion ", 3).unwrap(), 3);
        assert_eq!(inv.add("potion", 2).unwrap(), 5);
        assert!(inv.has("potion", 5));
        assert!(!inv.has("potion", 6));
        assert!(inv.remove("potion", 6).is_err());
        assert_eq!(inv.count("potion"), 5);
        assert_eq!(inv.remove("potion", 5).unwrap(), 0);
        assert!(inv.is_empty());
        assert!(inv.add("", 1).is_err());
        assert!(inv.add("x", 0).is_err());
        assert!(inv.add(&"n".repeat(MAX_NAME_CHARS + 1), 1).is_err());
    }

    #[test]
    fn metadata_rename_and_transfer() {
        let mut inv = GridInventory::default();
        inv.add("sword", 1).unwrap();
        inv.set_meta("sword", "damage", json!(7)).unwrap();
        assert!(inv.set_meta("shield", "x", json!(1)).is_err());
        inv.rename("sword", "blade").unwrap();
        assert_eq!(inv.get("blade").unwrap().meta["damage"], json!(7));

        let mut chest = GridInventory::default();
        inv.transfer(&mut chest, "blade", 1).unwrap();
        assert!(inv.is_empty());
        assert_eq!(chest.count("blade"), 1);
        assert_eq!(chest.get("blade").unwrap().meta["damage"], json!(7));
        assert!(inv.transfer(&mut chest, "blade", 1).is_err());

        chest.set("gold", 40).unwrap();
        assert_eq!(chest.total(), 41);
        chest.set("gold", 0).unwrap();
        assert_eq!(chest.len(), 1);

        let text = serde_json::to_string(&chest).unwrap();
        let back: GridInventory = serde_json::from_str(&text).unwrap();
        assert_eq!(back, chest);
    }
}
