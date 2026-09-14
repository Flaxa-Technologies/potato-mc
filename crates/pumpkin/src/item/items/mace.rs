use crate::entity::player::Player;
use crate::item::{ItemBehaviour, ItemMetadata};
use pumpkin_data::enchantment::Enchantment;
use pumpkin_data::item::Item;
use pumpkin_util::GameMode;

pub struct MaceItem;

impl ItemMetadata for MaceItem {
    fn ids() -> Box<[u16]> {
        [Item::MACE.id].into()
    }
}

impl ItemBehaviour for MaceItem {
    fn can_mine(&self, player: &Player) -> bool {
        player.gamemode.load() != GameMode::Creative
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl MaceItem {
    /// Calculates the smash attack damage bonus given fall distance and density enchantment level.
    ///
    /// Fall distance curve (vanilla 1.21.4):
    /// - 0 to 3 blocks: +4 damage per block fallen
    /// - 3 to 8 blocks: +2 damage per block fallen (starts at 12)
    /// - > 8 blocks: +1 damage per block fallen (starts at 22)
    ///
    /// Density adds +0.5 * density_level * fall_distance.
    #[must_use]
    pub fn calculate_smash_damage(fall_distance: f64, density_level: u32) -> f64 {
        let base_smash = if fall_distance <= 3.0 {
            4.0 * fall_distance
        } else if fall_distance <= 8.0 {
            12.0 + 2.0 * (fall_distance - 3.0)
        } else {
            22.0 + (fall_distance - 8.0)
        };
        let density_bonus = 0.5 * density_level as f64 * fall_distance;
        base_smash + density_bonus
    }

    /// Checks if an enchantment is allowed to be applied to a Mace according to vanilla 1.21 rules.
    #[must_use]
    pub fn is_enchantment_allowed(enchantment: &'static Enchantment) -> bool {
        if enchantment == &Enchantment::DENSITY
            || enchantment == &Enchantment::BREACH
            || enchantment == &Enchantment::WIND_BURST
            || enchantment == &Enchantment::UNBREAKING
            || enchantment == &Enchantment::MENDING
            || enchantment == &Enchantment::VANISHING_CURSE
        {
            return true;
        }

        // Specifically disallowed on Mace:
        // Sharpness, Smite, Bane of Arthropods, Fire Aspect, Looting, Sweeping Edge, Knockback, Impaling
        false
    }

    /// Checks if two enchantments can co-exist on a Mace.
    ///
    /// Rules:
    /// - Density is mutually exclusive with Breach, Smite, Bane of Arthropods, Sharpness, Impaling.
    /// - Breach is mutually exclusive with Density, Smite, Bane of Arthropods, Sharpness, Impaling.
    /// - Wind Burst is compatible with both Density and Breach.
    #[must_use]
    pub fn are_enchantments_compatible(a: &'static Enchantment, b: &'static Enchantment) -> bool {
        if a == b {
            return true;
        }

        let is_damage_enc = |e: &'static Enchantment| {
            e == &Enchantment::DENSITY
                || e == &Enchantment::BREACH
                || e == &Enchantment::SHARPNESS
                || e == &Enchantment::SMITE
                || e == &Enchantment::BANE_OF_ARTHROPODS
                || e == &Enchantment::IMPALING
        };

        if is_damage_enc(a) && is_damage_enc(b) {
            return false;
        }

        a.are_compatible(b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smash_damage_curve() {
        // Tier 1: 0 - 3 blocks (+4/block)
        assert_eq!(MaceItem::calculate_smash_damage(1.0, 0), 4.0);
        assert_eq!(MaceItem::calculate_smash_damage(2.0, 0), 8.0);
        assert_eq!(MaceItem::calculate_smash_damage(3.0, 0), 12.0);

        // Tier 2: 3 - 8 blocks (12 + 2/block)
        assert_eq!(MaceItem::calculate_smash_damage(5.0, 0), 16.0);
        assert_eq!(MaceItem::calculate_smash_damage(8.0, 0), 22.0);

        // Tier 3: > 8 blocks (22 + 1/block)
        assert_eq!(MaceItem::calculate_smash_damage(9.0, 0), 23.0);
        assert_eq!(MaceItem::calculate_smash_damage(12.0, 0), 26.0);
    }

    #[test]
    fn test_density_bonus() {
        // Fall 4 blocks with Density 5:
        // Base smash at 4 blocks = 12 + 2 * 1 = 14.0
        // Density bonus = 0.5 * 5 * 4 = 10.0
        // Total = 24.0
        assert_eq!(MaceItem::calculate_smash_damage(4.0, 5), 24.0);

        // Fall 10 blocks with Density 3:
        // Base smash at 10 blocks = 22 + (10 - 8) = 24.0
        // Density bonus = 0.5 * 3 * 10 = 15.0
        // Total = 39.0
        assert_eq!(MaceItem::calculate_smash_damage(10.0, 3), 39.0);
    }

    #[test]
    fn test_mace_enchantment_compatibility() {
        // Allowed enchantments
        assert!(MaceItem::is_enchantment_allowed(&Enchantment::DENSITY));
        assert!(MaceItem::is_enchantment_allowed(&Enchantment::BREACH));
        assert!(MaceItem::is_enchantment_allowed(&Enchantment::WIND_BURST));
        assert!(MaceItem::is_enchantment_allowed(&Enchantment::UNBREAKING));
        assert!(MaceItem::is_enchantment_allowed(&Enchantment::MENDING));
        assert!(MaceItem::is_enchantment_allowed(&Enchantment::VANISHING_CURSE));

        // Disallowed enchantments
        assert!(!MaceItem::is_enchantment_allowed(&Enchantment::SHARPNESS));
        assert!(!MaceItem::is_enchantment_allowed(&Enchantment::SMITE));
        assert!(!MaceItem::is_enchantment_allowed(&Enchantment::BANE_OF_ARTHROPODS));
        assert!(!MaceItem::is_enchantment_allowed(&Enchantment::FIRE_ASPECT));
        assert!(!MaceItem::is_enchantment_allowed(&Enchantment::LOOTING));
        assert!(!MaceItem::is_enchantment_allowed(&Enchantment::SWEEPING_EDGE));
        assert!(!MaceItem::is_enchantment_allowed(&Enchantment::KNOCKBACK));
        assert!(!MaceItem::is_enchantment_allowed(&Enchantment::IMPALING));

        // Mutual exclusivity
        assert!(!MaceItem::are_enchantments_compatible(&Enchantment::DENSITY, &Enchantment::BREACH));
        assert!(!MaceItem::are_enchantments_compatible(&Enchantment::BREACH, &Enchantment::DENSITY));
        assert!(!MaceItem::are_enchantments_compatible(&Enchantment::DENSITY, &Enchantment::SHARPNESS));
        assert!(!MaceItem::are_enchantments_compatible(&Enchantment::BREACH, &Enchantment::SMITE));

        // Wind Burst compatibility
        assert!(MaceItem::are_enchantments_compatible(&Enchantment::WIND_BURST, &Enchantment::DENSITY));
        assert!(MaceItem::are_enchantments_compatible(&Enchantment::DENSITY, &Enchantment::WIND_BURST));
        assert!(MaceItem::are_enchantments_compatible(&Enchantment::WIND_BURST, &Enchantment::BREACH));
        assert!(MaceItem::are_enchantments_compatible(&Enchantment::BREACH, &Enchantment::WIND_BURST));
        assert!(MaceItem::are_enchantments_compatible(&Enchantment::WIND_BURST, &Enchantment::UNBREAKING));
    }
}
