#![allow(
    unused,
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::cargo,
    clippy::undocumented_unsafe_blocks,
    clippy::if_then_some_else_none,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic
)]

#[rustfmt::skip]
#[path = "generated/chunk_view_lut.rs"]
pub mod chunk_view_lut;

#[rustfmt::skip]
#[path = "generated/loot_table.rs"]
pub mod loot_table;
pub use loot_table as chest_loot_table;

#[cfg(feature = "item")]
#[rustfmt::skip]
#[path = "generated/item.rs"]
pub mod item;

#[cfg(feature = "item")]
pub mod item_stack;

#[cfg(feature = "packet")]
#[rustfmt::skip]
#[path = "generated/packet.rs"]
pub mod packet;

#[cfg(feature = "jukebox_song")]
#[rustfmt::skip]
#[path = "generated/jukebox_song.rs"]
pub mod jukebox_song;

#[cfg(feature = "translation")]
#[rustfmt::skip]
#[path = "generated/translation.rs"]
pub mod translation;

#[cfg(feature = "registry")]
#[rustfmt::skip]
#[path = "generated/registry.rs"]
pub mod registry;

#[cfg(feature = "screen")]
#[rustfmt::skip]
#[path = "generated/screen.rs"]
pub mod screen;

#[cfg(feature = "particle")]
#[rustfmt::skip]
#[path = "generated/particle.rs"]
pub mod particle;

#[cfg(feature = "statistic")]
#[rustfmt::skip]
#[path = "generated/statistic.rs"]
pub mod statistic;

#[cfg(feature = "sound")]
#[rustfmt::skip]
#[path = "generated/sound_category.rs"]
mod sound_category;

#[cfg(feature = "sound")]
#[rustfmt::skip]
#[path = "generated/sound.rs"]
mod sound_enum;

#[cfg(feature = "sound")]
pub mod sound {
    pub use crate::sound_category::*;
    pub use crate::sound_enum::*;
}

#[cfg(feature = "advancement")]
#[rustfmt::skip]
#[path = "generated/advancement.rs"]
pub mod advancement;

#[cfg(feature = "advancement")]
pub mod advancement_data;

#[cfg(feature = "advancement")]
pub use advancement::*;

#[cfg(feature = "recipes")]
#[rustfmt::skip]
#[path = "generated/recipes.rs"]
pub mod recipes;

#[cfg(feature = "data_component")]
#[rustfmt::skip]
#[path = "generated/data_component.rs"]
pub mod data_component;

#[cfg(feature = "data_component")]
pub mod data_component_impl;

#[cfg(feature = "attributes")]
#[rustfmt::skip]
#[path = "generated/attributes.rs"]
pub mod attributes;

#[cfg(feature = "tracked_data")]
#[rustfmt::skip]
#[path = "generated/tracked_data.rs"]
pub mod tracked_data;

#[cfg(feature = "meta_data_type")]
#[rustfmt::skip]
#[path = "generated/meta_data_type.rs"]
pub mod meta_data_type;

#[cfg(feature = "noise_parameter")]
#[rustfmt::skip]
#[path = "generated/noise_parameter.rs"]
pub mod noise_parameter;

#[cfg(feature = "biome")]
#[expect(clippy::unreachable)]
#[rustfmt::skip]
#[path = "generated/biome.rs"]
pub mod biome;

#[cfg(feature = "biome")]
pub mod biome_remap;

#[cfg(feature = "chunk_status")]
#[rustfmt::skip]
#[path = "generated/chunk_status.rs"]
pub mod chunk_status;

#[cfg(feature = "chunk")]
pub mod chunk {
    #[cfg(feature = "biome")]
    pub use super::biome::*;
    #[cfg(feature = "chunk_status")]
    pub use super::chunk_status::ChunkStatus;
    #[cfg(feature = "noise_parameter")]
    pub use super::noise_parameter::*;
}

#[cfg(feature = "game_event")]
#[rustfmt::skip]
#[path = "generated/game_event.rs"]
pub mod game_event;

#[cfg(feature = "game_rules")]
#[rustfmt::skip]
#[path ="generated/game_rules.rs"]
pub mod game_rules;

#[cfg(feature = "entity_pose")]
#[rustfmt::skip]
#[path = "generated/entity_pose.rs"]
mod entity_pose;

#[cfg(feature = "entity_status")]
#[rustfmt::skip]
#[path = "generated/entity_status.rs"]
pub mod entity_status;

#[cfg(feature = "entity_type")]
#[rustfmt::skip]
#[path = "generated/entity_type.rs"]
mod entity_type;

#[cfg(feature = "spawn_egg")]
#[rustfmt::skip]
#[path = "generated/spawn_egg.rs"]
mod spawn_egg;

#[cfg(feature = "dimension")]
#[rustfmt::skip]
#[path = "generated/dimension.rs"]
pub mod dimension;

#[cfg(feature = "environment_attribute")]
#[rustfmt::skip]
#[path = "generated/environment_attribute.rs"]
pub mod environment_attribute;

#[cfg(feature = "environment_attribute")]
pub use environment_attribute::*;

#[cfg(feature = "enchantment")]
#[rustfmt::skip]
#[path = "generated/enchantment.rs"]
pub mod enchantment;

#[cfg(feature = "enchantment")]
pub use enchantment::*;

#[cfg(feature = "entity")]
pub mod entity {
    #[cfg(feature = "entity_pose")]
    pub use super::entity_pose::*;
    #[cfg(feature = "entity_status")]
    pub use super::entity_status::*;
    #[cfg(feature = "entity_type")]
    pub use super::entity_type::*;
    #[cfg(feature = "spawn_egg")]
    pub use super::spawn_egg::*;
}

#[cfg(feature = "world_event")]
#[rustfmt::skip]
#[path = "generated/world_event.rs"]
mod world_event;

#[cfg(feature = "message_type")]
#[rustfmt::skip]
#[path = "generated/message_type.rs"]
mod message_type;

#[cfg(feature = "world")]
pub mod world {
    #[cfg(feature = "message_type")]
    pub use super::message_type::*;
    #[cfg(feature = "world_event")]
    pub use super::world_event::*;
}

#[rustfmt::skip]
#[path = "generated/placed_feature.rs"]
pub mod placed_feature;

#[rustfmt::skip]
#[path = "generated/configured_feature.rs"]
pub mod configured_feature;

#[cfg(feature = "scoreboard")]
#[rustfmt::skip]
#[path = "generated/scoreboard_slot.rs"]
pub mod scoreboard;

#[cfg(feature = "damage")]
#[rustfmt::skip]
#[path = "generated/damage_type.rs"]
pub mod damage;

#[cfg(feature = "fluid")]
#[rustfmt::skip]
#[path = "generated/fluid.rs"]
pub mod fluid;

#[cfg(feature = "block")]
#[expect(clippy::unreachable)]
#[rustfmt::skip]
#[path = "generated/block.rs"]
pub mod block_properties;

#[cfg(feature = "block")]
#[rustfmt::skip]
#[path = "generated/block_state_remap.rs"]
pub mod block_state_remap_generated;

#[cfg(feature = "block")]
pub mod block_state_remap;

#[cfg(feature = "item_id_remap")]
#[rustfmt::skip]
#[path = "generated/item_id_remap.rs"]
pub mod item_id_remap;

#[cfg(feature = "entity_id_remap")]
#[rustfmt::skip]
#[path = "generated/entity_id_remap.rs"]
pub mod entity_id_remap;

#[cfg(feature = "sound_id_remap")]
#[rustfmt::skip]
#[path = "generated/sound_id_remap.rs"]
pub mod sound_id_remap;

#[cfg(feature = "particle_id_remap")]
#[rustfmt::skip]
#[path = "generated/particle_id_remap.rs"]
pub mod particle_id_remap;

#[cfg(feature = "menu_id_remap")]
#[rustfmt::skip]
#[path = "generated/menu_id_remap.rs"]
pub mod menu_id_remap;

#[cfg(feature = "recipe_serializer_id_remap")]
#[rustfmt::skip]
#[path = "generated/recipe_serializer_id_remap.rs"]
pub mod recipe_serializer_id_remap;

#[cfg(feature = "argument_type_id_remap")]
#[rustfmt::skip]
#[path = "generated/argument_type_id_remap.rs"]
pub mod argument_type_id_remap;

#[cfg(feature = "attribute_id_remap")]
#[rustfmt::skip]
#[path = "generated/attribute_id_remap.rs"]
pub mod attribute_id_remap;

#[cfg(feature = "block_entity_type_id_remap")]
#[rustfmt::skip]
#[path = "generated/block_entity_type_id_remap.rs"]
pub mod block_entity_type_id_remap;

#[cfg(feature = "custom_stat_id_remap")]
#[rustfmt::skip]
#[path = "generated/custom_stat_id_remap.rs"]
pub mod custom_stat_id_remap;

#[cfg(feature = "data_component_type_id_remap")]
#[rustfmt::skip]
#[path = "generated/data_component_type_id_remap.rs"]
pub mod data_component_type_id_remap;

#[cfg(feature = "enchantment_id_remap")]
#[rustfmt::skip]
#[path = "generated/enchantment_id_remap.rs"]
pub mod enchantment_id_remap;

#[cfg(feature = "environment_attribute_id_remap")]
#[rustfmt::skip]
#[path = "generated/environment_attribute_id_remap.rs"]
pub mod environment_attribute_id_remap;

#[cfg(feature = "painting_variant_id_remap")]
#[rustfmt::skip]
#[path = "generated/painting_variant_id_remap.rs"]
pub mod painting_variant_id_remap;

#[cfg(feature = "slot_display_id_remap")]
#[rustfmt::skip]
#[path = "generated/slot_display_id_remap.rs"]
pub mod slot_display_id_remap;

#[cfg(feature = "bedrock_creative")]
#[rustfmt::skip]
#[path = "generated/bedrock_creative.rs"]
pub mod bedrock_creative;

#[cfg(feature = "bedrock_biome")]
#[rustfmt::skip]
#[path = "generated/bedrock_biome.rs"]
pub mod bedrock_biome;

#[cfg(feature = "tag")]
#[rustfmt::skip]
#[path = "generated/tag.rs"]
pub mod tag;

#[cfg(feature = "noise_router")]
#[rustfmt::skip]
#[path = "generated/noise_router.rs"]
pub mod noise_router;

#[cfg(feature = "composter")]
#[rustfmt::skip]
#[path = "generated/composter_increase_chance.rs"]
pub mod composter_increase_chance;

#[cfg(feature = "flower_pot")]
#[rustfmt::skip]
#[path = "generated/flower_pot_transformations.rs"]
pub mod flower_pot_transformations;

#[cfg(feature = "fuels")]
#[rustfmt::skip]
#[path = "generated/fuels.rs"]
pub mod fuels;

#[cfg(feature = "effect")]
#[rustfmt::skip]
#[path = "generated/effect.rs"]
pub mod effect;

#[cfg(feature = "effect")]
#[rustfmt::skip]
#[path = "generated/status_effect.rs"]
pub mod status_effect;

#[cfg(feature = "structures")]
#[rustfmt::skip]
#[path = "generated/structures.rs"]
pub mod structures;

#[cfg(feature = "potion")]
#[rustfmt::skip]
#[path = "generated/potion.rs"]
pub mod potion;

#[cfg(feature = "potion_brewing")]
#[rustfmt::skip]
#[path = "generated/potion_brewing.rs"]
pub mod potion_brewing;

#[cfg(feature = "recipe_remainder")]
#[rustfmt::skip]
#[path = "generated/recipe_remainder.rs"]
pub mod recipe_remainder;

#[cfg(feature = "block")]
mod block_direction;
#[cfg(feature = "block")]
pub mod block_rotation;
#[cfg(feature = "block")]
pub mod block_state;
#[cfg(feature = "block")]
mod blocks;

#[cfg(feature = "block")]
pub use block_direction::{BlockDirection, FacingExt, HorizontalFacingExt};
#[cfg(feature = "block")]
pub use block_rotation::{Mirror, Rotation, transform_block_properties, transform_rail_shape};
#[cfg(feature = "block")]
pub use block_state::{BlockState, BlockStateId};
#[cfg(feature = "block")]
pub use blocks::{Block, BlockId};

#[cfg(feature = "material_rule")]
#[rustfmt::skip]
#[path = "generated/material_rule.rs"]
pub mod material_rule;

#[cfg(feature = "noise_settings")]
#[rustfmt::skip]
#[path = "generated/noise_settings.rs"]
pub mod noise_settings;

#[cfg(feature = "chunk_gen_settings")]
pub use noise_settings as chunk_gen_settings;

#[cfg(feature = "carver")]
#[rustfmt::skip]
#[path = "generated/carver.rs"]
pub mod carver;

#[cfg(feature = "villager")]
#[rustfmt::skip]
#[path = "generated/villager.rs"]
pub mod villager;

#[cfg(feature = "slot_ranges")]
#[rustfmt::skip]
#[path = "generated/slot_ranges.rs"]
pub mod slot_ranges;

#[cfg(feature = "map_color")]
#[rustfmt::skip]
#[path = "generated/map_color.rs"]
pub mod map_color;

#[cfg(feature = "map_decoration")]
#[rustfmt::skip]
#[path = "generated/map_decoration.rs"]
pub mod map_decoration;

#[cfg(feature = "dye_color")]
#[rustfmt::skip]
#[path = "generated/dye_color.rs"]
pub mod dye_color;

#[cfg(feature = "block_transformer")]
#[rustfmt::skip]
#[path = "generated/block_transformer.rs"]
pub mod block_transformer;

#[cfg(feature = "trial_spawner")]
#[rustfmt::skip]
#[path = "generated/trial_spawner.rs"]
pub mod trial_spawner;

#[cfg(test)]
mod tests {
    use super::*;
    use pumpkin_util::version::JavaMinecraftVersion;
    use std::io::Cursor;

    #[test]
    fn test_26_3_trim_material_and_biome_effects() {
        let registries = registry::Registry::get_synced(JavaMinecraftVersion::V_26_3);

        // 1. Verify trim_material description color is a String (e.g. "#9A5CC6"), NOT an Int!
        let trim_mat = registries
            .iter()
            .find(|r| r.registry_id == "minecraft:trim_material")
            .expect("trim_material registry must exist in 26.3");
        let amethyst = trim_mat
            .registry_entries
            .iter()
            .find(|e| e.entry_id == "minecraft:amethyst")
            .expect("amethyst entry must exist");
        let data = amethyst.data.as_ref().expect("amethyst data must exist");

        let mut cursor = Cursor::new(&data[..]);
        let mut reader = pumpkin_nbt::deserializer::NbtReadHelperJava::new(&mut cursor);
        let nbt = pumpkin_nbt::Nbt::read_unnamed(&mut reader).expect("valid amethyst NBT");
        let desc = nbt
            .root_tag
            .get_compound("description")
            .expect("description compound in amethyst");
        let color = desc
            .get_string("color")
            .expect("color must be string in trim_material description");
        assert_eq!(color, "#9A5CC6");

        // 2. Verify biome effects colors are Ints
        let biomes = registries
            .iter()
            .find(|r| r.registry_id == "minecraft:worldgen/biome")
            .expect("worldgen/biome registry must exist in 26.3");
        let plains = biomes
            .registry_entries
            .iter()
            .find(|e| e.entry_id == "minecraft:plains")
            .expect("plains entry must exist");
        let p_data = plains.data.as_ref().expect("plains data must exist");

        let mut p_cursor = Cursor::new(&p_data[..]);
        let mut p_reader = pumpkin_nbt::deserializer::NbtReadHelperJava::new(&mut p_cursor);
        let p_nbt = pumpkin_nbt::Nbt::read_unnamed(&mut p_reader).expect("valid plains NBT");
        let effects = p_nbt
            .root_tag
            .get_compound("effects")
            .expect("effects compound in plains");
        let water_color = effects
            .get_int("water_color")
            .expect("water_color must be int in biome effects");
        assert_eq!(water_color, 4159204); // 0x3f76e4 = 4159204

        // 3. Verify 26.3 dappled_forest visual attributes colors are Ints
        let dappled = biomes
            .registry_entries
            .iter()
            .find(|e| e.entry_id == "minecraft:dappled_forest")
            .expect("dappled_forest entry must exist");
        let d_data = dappled.data.as_ref().expect("dappled_forest data must exist");

        let mut d_cursor = Cursor::new(&d_data[..]);
        let mut d_reader = pumpkin_nbt::deserializer::NbtReadHelperJava::new(&mut d_cursor);
        let d_nbt = pumpkin_nbt::Nbt::read_unnamed(&mut d_reader).expect("valid dappled_forest NBT");
        let attributes = d_nbt
            .root_tag
            .get_compound("attributes")
            .expect("attributes compound in dappled_forest");
        let sky_color = attributes
            .get_int("minecraft:visual/sky_color")
            .expect("minecraft:visual/sky_color must be int in dappled_forest attributes");
        assert_eq!(sky_color, 8168447); // #7ca3ff = 0x7ca3ff = 8168447

        let fog_color = attributes
            .get_int("minecraft:visual/fog_color")
            .expect("minecraft:visual/fog_color must be int in dappled_forest attributes");
        assert_eq!(fog_color, 13424866); // #ccd8e2 = 0xccd8e2 = 13424866
    }

    #[test]
    fn test_26_3_wool_slabs_and_stairs_remapping_and_blocks() {
        use crate::block_state_remap::remap_block_state_for_version;
        use crate::item::Item;
        use crate::tag::Taggable;
        use crate::Block;
        use pumpkin_util::version::JavaMinecraftVersion;

        // 1. Verify block lookup from item id
        let lime_slab_block = Block::from_item_id(Item::LIME_WOOL_SLAB.id)
            .expect("Lime wool slab item must map to a block");
        assert_eq!(lime_slab_block.name, "lime_wool_slab");
        assert_eq!(lime_slab_block.id, Block::LIME_WOOL_SLAB.id);

        let lime_stairs_block = Block::from_item_id(Item::LIME_WOOL_STAIRS.id)
            .expect("Lime wool stairs item must map to a block");
        assert_eq!(lime_stairs_block.name, "lime_wool_stairs");
        assert_eq!(lime_stairs_block.id, Block::LIME_WOOL_STAIRS.id);

        // 2. Verify tag membership
        assert_eq!(
            lime_slab_block.is_tagged_with("minecraft:slabs"),
            Some(true),
            "Lime wool slab must be in minecraft:slabs tag"
        );
        assert_eq!(
            lime_stairs_block.is_tagged_with("minecraft:stairs"),
            Some(true),
            "Lime wool stairs must be in minecraft:stairs tag"
        );

        // 3. Verify 26.3 block state remapping matches vanilla 26.3 reports
        let remapped_slab = remap_block_state_for_version(
            lime_slab_block.default_state.id.as_u16(),
            JavaMinecraftVersion::V_26_3,
        );
        assert_eq!(
            remapped_slab, 3738,
            "Lime wool slab default state must remap to 26.3 vanilla state 3738"
        );

        let remapped_stairs = remap_block_state_for_version(
            lime_stairs_block.default_state.id.as_u16(),
            JavaMinecraftVersion::V_26_3,
        );
        assert_eq!(
            remapped_stairs, 2836,
            "Lime wool stairs default state must remap to 26.3 vanilla state 2836"
        );

        // 4. Verify vanilla fire internal state 3738 is remapped via 26.2->26.3 table (NOT returning 3738)
        let fire_state = 3738u16;
        let remapped_fire = remap_block_state_for_version(fire_state, JavaMinecraftVersion::V_26_3);
        assert_ne!(
            remapped_fire, 3738,
            "Internal fire state 3738 must not be bypassed or confused with 26.3 wool slab"
        );

        // 5. Verify fallback for older clients
        let old_client_state = remap_block_state_for_version(
            lime_slab_block.default_state.id.as_u16(),
            JavaMinecraftVersion::V_1_21_4,
        );
        assert_eq!(
            old_client_state, 1,
            "Older clients must fallback to solid block (1) for 26.3 wool slabs"
        );
    }

    #[test]
    fn test_26_3_additional_features_blocks_items_tags() {
        use crate::block_state_remap::remap_block_state_for_version;
        use crate::item::Item;
        use crate::tag::Taggable;
        use pumpkin_util::version::JavaMinecraftVersion;

        // 1. Straw Bed
        let straw_bed = Block::STRAW_BED;
        assert_eq!(straw_bed.id.as_u16(), 1228);
        assert_eq!(Item::STRAW_BED.id, 1569);
        assert_eq!(Block::from_item_id(1569), Some(&straw_bed));
        assert_eq!(
            remap_block_state_for_version(straw_bed.default_state.id.as_u16(), JavaMinecraftVersion::V_26_3),
            2289
        );
        assert_eq!(
            remap_block_state_for_version(straw_bed.default_state.id.as_u16(), JavaMinecraftVersion::V_26_2),
            1
        );

        // 2. Shelf Mushroom & Red Shrub
        let shelf_mushroom = Block::SHELF_MUSHROOM;
        assert_eq!(shelf_mushroom.id.as_u16(), 1230);
        assert_eq!(Item::SHELF_MUSHROOM.id, 1571);
        assert_eq!(Block::from_item_id(1571), Some(&shelf_mushroom));
        assert_eq!(
            remap_block_state_for_version(shelf_mushroom.default_state.id.as_u16(), JavaMinecraftVersion::V_26_3),
            11227
        );
        assert_eq!(
            remap_block_state_for_version(shelf_mushroom.default_state.id.as_u16(), JavaMinecraftVersion::V_26_2),
            1
        );

        let red_shrub = Block::RED_SHRUB;
        assert_eq!(red_shrub.id.as_u16(), 1229);
        assert_eq!(Item::RED_SHRUB.id, 1570);
        assert_eq!(Block::from_item_id(1570), Some(&red_shrub));
        assert_eq!(
            remap_block_state_for_version(red_shrub.default_state.id.as_u16(), JavaMinecraftVersion::V_26_3),
            2367
        );
        assert_eq!(
            remap_block_state_for_version(red_shrub.default_state.id.as_u16(), JavaMinecraftVersion::V_26_2),
            1
        );

        // 3. Concrete Stairs & Slabs
        let white_concrete_stairs = Block::WHITE_CONCRETE_STAIRS;
        assert_eq!(white_concrete_stairs.id.as_u16(), 1231);
        assert_eq!(Item::WHITE_CONCRETE_STAIRS.id, 1572);
        assert_eq!(Block::from_item_id(1572), Some(&white_concrete_stairs));
        assert_eq!(white_concrete_stairs.is_tagged_with("minecraft:stairs"), Some(true));

        let white_concrete_slab = Block::WHITE_CONCRETE_SLAB;
        assert_eq!(white_concrete_slab.id.as_u16(), 1247);
        assert_eq!(Item::WHITE_CONCRETE_SLAB.id, 1588);
        assert_eq!(Block::from_item_id(1588), Some(&white_concrete_slab));
        assert_eq!(white_concrete_slab.is_tagged_with("minecraft:slabs"), Some(true));

        // 4. Poplar Wood Set
        let poplar_planks = Block::POPLAR_PLANKS;
        assert_eq!(poplar_planks.id.as_u16(), 1263);
        assert_eq!(Item::POPLAR_PLANKS.id, 1604);
        assert_eq!(Block::from_item_id(1604), Some(&poplar_planks));
        assert_eq!(poplar_planks.is_tagged_with("minecraft:planks"), Some(true));

        let poplar_stairs = Block::POPLAR_STAIRS;
        assert_eq!(poplar_stairs.id.as_u16(), 1281);
        assert_eq!(Item::POPLAR_STAIRS.id, 1619);
        assert_eq!(Block::from_item_id(1619), Some(&poplar_stairs));
        assert_eq!(poplar_stairs.is_tagged_with("minecraft:stairs"), Some(true));
        assert_eq!(poplar_stairs.is_tagged_with("minecraft:wooden_stairs"), Some(true));

        let poplar_slab = Block::POPLAR_SLAB;
        assert_eq!(poplar_slab.id.as_u16(), 1282);
        assert_eq!(Item::POPLAR_SLAB.id, 1620);
        assert_eq!(Block::from_item_id(1620), Some(&poplar_slab));
        assert_eq!(poplar_slab.is_tagged_with("minecraft:slabs"), Some(true));
        assert_eq!(poplar_slab.is_tagged_with("minecraft:wooden_slabs"), Some(true));

        let poplar_door = Block::POPLAR_DOOR;
        assert_eq!(poplar_door.id.as_u16(), 1285);
        assert_eq!(Item::POPLAR_DOOR.id, 1623);
        assert_eq!(Block::from_item_id(1623), Some(&poplar_door));
        assert_eq!(poplar_door.is_tagged_with("minecraft:doors"), Some(true));

        // Boats
        assert_eq!(Item::POPLAR_BOAT.id, 1624);
        assert_eq!(Item::POPLAR_CHEST_BOAT.id, 1625);

        // 5. Cushions
        assert_eq!(Item::WHITE_CUSHION.id, 1626);
        assert_eq!(Item::BLACK_CUSHION.id, 1641);

        // 6. Explorer Maps (16 items: 1642..=1657) & extendable_maps restriction
        assert_eq!(Item::ABANDONED_CAMP_MAP.id, 1642);
        assert_eq!(Item::WOODLAND_MANSION_MAP.id, 1657);
        assert_eq!(Item::FILLED_MAP.is_tagged_with("minecraft:extendable_maps"), Some(true));
        assert_eq!(Item::ABANDONED_CAMP_MAP.is_tagged_with("minecraft:extendable_maps"), Some(false));
    }
}

