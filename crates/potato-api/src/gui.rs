use crate::types::ItemStack;

/// Represents a custom virtual GUI container (e.g. custom chest, shop GUI, or menu).
#[derive(Debug, Clone)]
pub struct Gui {
    pub title: String,
    pub size: usize,
    pub items: Vec<Option<ItemStack>>,
    pub allow_grab_items: bool,
    pub allow_put_items: bool,
}

impl Gui {
    /// Creates a new virtual GUI with the specified title and slot count.
    /// Standard chest sizes: 9, 18, 27, 36, 45, 54.
    pub fn new(title: impl Into<String>, size: usize) -> Self {
        let actual_size = size.max(1);
        Self {
            title: title.into(),
            size: actual_size,
            items: vec![None; actual_size],
            allow_grab_items: false,
            allow_put_items: false,
        }
    }

    /// Creates a standard chest menu with a specified number of 9-slot rows (1 to 6 rows).
    pub fn chest(title: impl Into<String>, rows: usize) -> Self {
        let r = rows.clamp(1, 6);
        Self::new(title, r * 9)
    }

    /// Creates a 5-slot hopper menu.
    pub fn hopper(title: impl Into<String>) -> Self {
        Self::new(title, 5)
    }

    /// Creates a 3x3 (9 slots) dispenser or dropper menu.
    pub fn dispenser(title: impl Into<String>) -> Self {
        Self::new(title, 9)
    }

    /// Sets an item at the specified slot index (0-indexed).
    pub fn set_item(&mut self, slot: usize, item: Option<ItemStack>) -> &mut Self {
        if slot < self.items.len() {
            self.items[slot] = item;
        }
        self
    }

    /// Retrieves a reference to the item at the specified slot index.
    pub fn get_item(&self, slot: usize) -> Option<&ItemStack> {
        self.items.get(slot).and_then(|i| i.as_ref())
    }

    /// Sets whether players are allowed to take/grab items out of this GUI.
    pub fn allow_grab(&mut self, allow: bool) -> &mut Self {
        self.allow_grab_items = allow;
        self
    }

    /// Sets whether players are allowed to insert/put items into this GUI.
    pub fn allow_put(&mut self, allow: bool) -> &mut Self {
        self.allow_put_items = allow;
        self
    }

    /// Clears all items from this GUI.
    pub fn clear(&mut self) {
        for slot in &mut self.items {
            *slot = None;
        }
    }
}
