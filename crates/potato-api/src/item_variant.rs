use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use serde::{Deserialize, Serialize};

use crate::types::ItemStack;

/// Declarative template for a resource-pack-driven item variant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemVariant {
    pub base_item: String,
    pub custom_model_data: Option<i32>,
    pub name: Option<String>,
    pub lore: Vec<String>,
    pub enchantments: HashMap<String, u32>,
    pub persistent_tags: HashMap<String, String>,
    pub unbreakable: bool,
}

impl ItemVariant {
    /// Creates a new variant builder based on a vanilla Minecraft item ID.
    pub fn builder(base_item: impl Into<String>) -> ItemVariantBuilder {
        ItemVariantBuilder::new(base_item)
    }

    /// Instantiates an `ItemStack` corresponding to this variant.
    pub fn create(&self, amount: u32) -> ItemStack {
        let mut item = ItemStack::new(&self.base_item, amount);
        if let Some(cmd) = self.custom_model_data {
            item.set_custom_model_data(Some(cmd));
        }
        if let Some(ref name) = self.name {
            item.set_custom_name(name.clone());
        }
        for l in &self.lore {
            item.add_lore(l.clone());
        }
        for (ench, lvl) in &self.enchantments {
            item.add_enchantment(ench.clone(), *lvl);
        }
        for (k, v) in &self.persistent_tags {
            item.pdc_mut().set_string(k.clone(), v.clone());
        }
        if self.unbreakable {
            item.pdc_mut().set_string("Unbreakable", "true");
        }
        item
    }

    /// Checks if a given `ItemStack` matches this variant definition.
    pub fn matches(&self, item: &ItemStack) -> bool {
        // Base item check
        let item_base = item.item_type.strip_prefix("minecraft:").unwrap_or(&item.item_type);
        let self_base = self.base_item.strip_prefix("minecraft:").unwrap_or(&self.base_item);
        if !item_base.eq_ignore_ascii_case(self_base) {
            return false;
        }

        // Custom Model Data check
        if self.custom_model_data != item.custom_model_data {
            return false;
        }

        // Persistent tag verification
        for (k, v) in &self.persistent_tags {
            if item.pdc().get_string(k) != Some(v.as_str()) {
                return false;
            }
        }

        true
    }
}

/// Fluent builder for creating an `ItemVariant`.
#[derive(Debug, Clone, Default)]
pub struct ItemVariantBuilder {
    base_item: String,
    custom_model_data: Option<i32>,
    name: Option<String>,
    lore: Vec<String>,
    enchantments: HashMap<String, u32>,
    persistent_tags: HashMap<String, String>,
    unbreakable: bool,
}

impl ItemVariantBuilder {
    pub fn new(base_item: impl Into<String>) -> Self {
        Self {
            base_item: base_item.into(),
            ..Default::default()
        }
    }

    pub fn custom_model_data(mut self, cmd: i32) -> Self {
        self.custom_model_data = Some(cmd);
        self
    }

    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn lore<I, S>(mut self, lines: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        for line in lines {
            self.lore.push(line.into());
        }
        self
    }

    pub fn add_lore_line(mut self, line: impl Into<String>) -> Self {
        self.lore.push(line.into());
        self
    }

    pub fn enchantment(mut self, enchantment: impl Into<String>, level: u32) -> Self {
        self.enchantments.insert(enchantment.into(), level);
        self
    }

    pub fn persistent_tag(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.persistent_tags.insert(key.into(), value.into());
        self
    }

    pub fn unbreakable(mut self, unbreakable: bool) -> Self {
        self.unbreakable = unbreakable;
        self
    }

    pub fn build(self) -> ItemVariant {
        ItemVariant {
            base_item: self.base_item,
            custom_model_data: self.custom_model_data,
            name: self.name,
            lore: self.lore,
            enchantments: self.enchantments,
            persistent_tags: self.persistent_tags,
            unbreakable: self.unbreakable,
        }
    }
}

/// Central registry for managing resource-pack-driven item variants.
#[derive(Debug, Clone, Default)]
pub struct ItemVariantRegistry {
    variants: Arc<RwLock<HashMap<String, ItemVariant>>>,
}

impl ItemVariantRegistry {
    pub fn new() -> Self {
        Self {
            variants: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Registers a custom item variant with a unique key (e.g. "mmo:ruby_sword").
    pub fn register(&self, key: impl Into<String>, variant: ItemVariant) {
        let mut map = self.variants.write().unwrap();
        map.insert(key.into(), variant);
    }

    /// Looks up a registered variant by key.
    pub fn get(&self, key: &str) -> Option<ItemVariant> {
        let map = self.variants.read().unwrap();
        map.get(key).cloned()
    }

    /// Creates an `ItemStack` for the registered variant key.
    pub fn create(&self, key: &str, amount: u32) -> Option<ItemStack> {
        self.get(key).map(|v| v.create(amount))
    }

    /// Checks whether an `ItemStack` matches the given variant key.
    pub fn matches(&self, item: &ItemStack, key: &str) -> bool {
        if let Some(variant) = self.get(key) {
            variant.matches(item)
        } else {
            false
        }
    }

    /// Identifies an `ItemStack` by finding any matching registered variant key.
    pub fn identify(&self, item: &ItemStack) -> Option<String> {
        let map = self.variants.read().unwrap();
        for (key, variant) in map.iter() {
            if variant.matches(item) {
                return Some(key.clone());
            }
        }
        None
    }
}
