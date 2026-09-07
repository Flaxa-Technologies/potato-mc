use std::sync::Arc;

use pumpkin_data::{
    Block, BlockStateId,
    block_properties::{BlockProperties, OakSaplingLikeProperties},
};
use pumpkin_macros::pumpkin_block_from_tag;
use pumpkin_util::math::position::BlockPos;
use pumpkin_world::world::BlockFlags;

use crate::block::blocks::plant::PlantBlockBase;
use crate::block::{
    BlockBehaviour, BonemealArgs, CanPlaceAtArgs, GetStateForNeighborUpdateArgs, RandomTickArgs,
};
use crate::plugin::api::events::world::structure_grow::{StructureGrowEvent, TreeType};
use crate::world::World;

#[pumpkin_block_from_tag("minecraft:saplings")]
pub struct SaplingBlock;

impl SaplingBlock {
    #[must_use]
    pub fn get_tree_type(block: &Block) -> TreeType {
        match block.name {
            "oak_sapling" => TreeType::Oak,
            "spruce_sapling" => TreeType::Spruce,
            "birch_sapling" => TreeType::Birch,
            "jungle_sapling" => TreeType::Jungle,
            "acacia_sapling" => TreeType::Acacia,
            "dark_oak_sapling" | "pale_oak_sapling" => TreeType::DarkOak,
            "cherry_sapling" => TreeType::Cherry,
            "azalea" | "flowering_azalea" => TreeType::Azalea,
            "mangrove_propagule" => TreeType::Mangrove,
            _ => TreeType::Custom,
        }
    }

    pub fn advance_tree(
        world: &Arc<World>,
        pos: &BlockPos,
        block: &Block,
        state_id: BlockStateId,
        bone_meal: bool,
    ) {
        if OakSaplingLikeProperties::handles_block_id(block.id) {
            let mut props = OakSaplingLikeProperties::from_state_id(state_id);
            if props.stage == 0 {
                props.stage = 1;
                world.set_block_state(pos, props.to_state_id(block), BlockFlags::NOTIFY_ALL);
                return;
            }
        }

        let tree_type = Self::get_tree_type(block);
        let mut event = StructureGrowEvent::new(*pos, tree_type, bone_meal);
        if let Some(server) = world.server.upgrade() {
            server.plugin_manager.fire_blocking(&server, &mut event);
        }
        if event.cancelled {
            return;
        }

        Self::grow_tree(world, pos, block);
    }

    fn find_2x2_origin(world: &World, pos: &BlockPos, block_id: pumpkin_data::BlockId) -> Option<BlockPos> {
        for dx in [0, -1] {
            for dz in [0, -1] {
                let origin = BlockPos::new(pos.0.x + dx, pos.0.y, pos.0.z + dz);
                if Self::is_matching_sapling(world, &origin, block_id)
                    && Self::is_matching_sapling(world, &origin.add(1, 0, 0), block_id)
                    && Self::is_matching_sapling(world, &origin.add(0, 0, 1), block_id)
                    && Self::is_matching_sapling(world, &origin.add(1, 0, 1), block_id)
                {
                    return Some(origin);
                }
            }
        }
        None
    }

    fn is_matching_sapling(world: &World, pos: &BlockPos, block_id: pumpkin_data::BlockId) -> bool {
        world.get_block_state_id(pos).to_block_id() == block_id
    }

    fn grow_tree(world: &Arc<World>, pos: &BlockPos, block: &Block) {
        use pumpkin_data::configured_feature::ConfiguredFeature as CKey;
        use pumpkin_util::random::{RandomGenerator, legacy_rand::LegacyRand};
        use pumpkin_world::generation::feature::configured_features::{
            CONFIGURED_FEATURES, ConfiguredFeature,
        };
        use crate::world::block_placer::{CommandBlockRegistry, WorldGenAdapter};

        let maybe_2x2 = match block.name {
            "dark_oak_sapling" | "spruce_sapling" | "jungle_sapling" => {
                Self::find_2x2_origin(world, pos, block.id)
            }
            _ => None,
        };

        let feature_key = match block.name {
            "dark_oak_sapling" => {
                if maybe_2x2.is_none() {
                    // Vanilla: Dark Oak requires 2x2 configuration to grow
                    return;
                }
                CKey::DarkOak
            }
            "spruce_sapling" => {
                if maybe_2x2.is_some() {
                    if rand::random::<bool>() {
                        CKey::MegaSpruce
                    } else {
                        CKey::MegaPine
                    }
                } else {
                    CKey::Spruce
                }
            }
            "jungle_sapling" => {
                if maybe_2x2.is_some() {
                    CKey::MegaJungleTree
                } else {
                    CKey::JungleTree
                }
            }
            "oak_sapling" => {
                if rand::random::<f32>() < 0.1 {
                    CKey::FancyOak
                } else {
                    CKey::Oak
                }
            }
            "birch_sapling" => CKey::Birch,
            "acacia_sapling" => CKey::Acacia,
            "cherry_sapling" => CKey::Cherry,
            "pale_oak_sapling" => CKey::PaleOak,
            "mangrove_propagule" => CKey::Mangrove,
            "azalea" | "flowering_azalea" => CKey::AzaleaTree,
            _ => CKey::Oak,
        };

        let Some(configured) = CONFIGURED_FEATURES.get(&feature_key) else {
            return;
        };
        let ConfiguredFeature::Tree(tree_feature) = configured else {
            return;
        };

        let grow_pos = maybe_2x2.unwrap_or(*pos);
        let reg = CommandBlockRegistry;
        let mut adapter = WorldGenAdapter::new(world);
        let mut random = RandomGenerator::Legacy(LegacyRand::from_seed(rand::random()));

        // Temporarily clear sapling(s) to AIR
        let air_id = Block::AIR.default_state.id;
        if maybe_2x2.is_some() {
            world.set_block_state(&grow_pos, air_id, BlockFlags::NOTIFY_ALL);
            world.set_block_state(&grow_pos.add(1, 0, 0), air_id, BlockFlags::NOTIFY_ALL);
            world.set_block_state(&grow_pos.add(0, 0, 1), air_id, BlockFlags::NOTIFY_ALL);
            world.set_block_state(&grow_pos.add(1, 0, 1), air_id, BlockFlags::NOTIFY_ALL);
        } else {
            world.set_block_state(pos, air_id, BlockFlags::NOTIFY_ALL);
        }

        let success = tree_feature.generate(&reg, &mut adapter, &mut random, grow_pos);

        if success {
            adapter.finalize();
        } else {
            // Restore sapling(s) on growth failure
            let sapling_id = block.default_state.id;
            if maybe_2x2.is_some() {
                world.set_block_state(&grow_pos, sapling_id, BlockFlags::NOTIFY_ALL);
                world.set_block_state(&grow_pos.add(1, 0, 0), sapling_id, BlockFlags::NOTIFY_ALL);
                world.set_block_state(&grow_pos.add(0, 0, 1), sapling_id, BlockFlags::NOTIFY_ALL);
                world.set_block_state(&grow_pos.add(1, 0, 1), sapling_id, BlockFlags::NOTIFY_ALL);
            } else {
                world.set_block_state(pos, sapling_id, BlockFlags::NOTIFY_ALL);
            }
        }
    }
}

impl BlockBehaviour for SaplingBlock {
    fn can_place_at(&self, args: CanPlaceAtArgs<'_>) -> bool {
        <Self as PlantBlockBase>::can_place_at(self, args.block_accessor, args.position)
    }

    fn get_state_for_neighbor_update(
        &self,
        args: GetStateForNeighborUpdateArgs<'_>,
    ) -> BlockStateId {
        <Self as PlantBlockBase>::get_state_for_neighbor_update(
            self,
            args.world,
            args.position,
            args.state_id,
        )
    }

    fn random_tick(&self, args: RandomTickArgs<'_>) {
        if rand::random::<u8>().is_multiple_of(7) {
            let state_id = args.world.get_block_state_id(args.position);
            Self::advance_tree(args.world, args.position, args.block, state_id, false);
        }
    }

    fn is_valid_bonemeal_target(&self, _args: BonemealArgs<'_>) -> bool {
        true
    }

    fn is_bonemeal_success(&self, _args: BonemealArgs<'_>) -> bool {
        rand::random::<f32>() < 0.45
    }

    fn perform_bonemeal(&self, args: BonemealArgs<'_>) {
        {
            Self::advance_tree(args.world, args.position, args.block, args.state_id, true);
        }
    }
}

impl PlantBlockBase for SaplingBlock {}
