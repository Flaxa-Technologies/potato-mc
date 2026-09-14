use crate::block::{BlockBehaviour, CanPlaceAtArgs, GetStateForNeighborUpdateArgs};
use pumpkin_data::{Block, BlockStateId};
use pumpkin_data::block_properties::is_air;
use pumpkin_macros::{pumpkin_block, pumpkin_block_from_tag};
use pumpkin_util::math::position::BlockPos;
use pumpkin_world::world::BlockAccessor;

#[pumpkin_block_from_tag("minecraft:wool_carpets")]
pub struct CarpetBlock;

impl BlockBehaviour for CarpetBlock {
    fn can_place_at(&self, args: CanPlaceAtArgs<'_>) -> bool {
        can_place_at(args.block_accessor, args.position)
    }

    fn get_state_for_neighbor_update(
        &self,
        args: GetStateForNeighborUpdateArgs<'_>,
    ) -> BlockStateId {
        if !can_place_at(args.world, args.position) {
            return Block::AIR.default_state.id;
        }
        args.state_id
    }
}

#[pumpkin_block("minecraft:moss_carpet")]
pub struct MossCarpetBlock;

impl BlockBehaviour for MossCarpetBlock {
    fn can_place_at(&self, args: CanPlaceAtArgs<'_>) -> bool {
        can_place_at(args.block_accessor, args.position)
    }

    fn get_state_for_neighbor_update(
        &self,
        args: GetStateForNeighborUpdateArgs<'_>,
    ) -> BlockStateId {
        if !can_place_at(args.world, args.position) {
            return Block::AIR.default_state.id;
        }
        args.state_id
    }
}

#[pumpkin_block("minecraft:pale_moss_carpet")]
pub struct PaleMossCarpetBlock;

impl BlockBehaviour for PaleMossCarpetBlock {
    fn can_place_at(&self, args: CanPlaceAtArgs<'_>) -> bool {
        can_place_at(args.block_accessor, args.position)
    }

    fn get_state_for_neighbor_update(
        &self,
        args: GetStateForNeighborUpdateArgs<'_>,
    ) -> BlockStateId {
        if !can_place_at(args.world, args.position) {
            return Block::AIR.default_state.id;
        }
        args.state_id
    }
}

fn can_place_at(block_accessor: &dyn BlockAccessor, block_pos: &BlockPos) -> bool {
    !is_air(block_accessor.get_block_state_id(&block_pos.down()))
}

#[cfg(test)]
mod tests {
    use crate::world::loot::LootContextParameters;
    use pumpkin_data::Block;

    #[test]
    fn carpet_drops_itself_without_player_or_tool() {
        let key = format!("minecraft:blocks/{}", Block::RED_CARPET.name);
        let loot_table = pumpkin_data::loot_table::get_loot_table(&key)
            .expect("Red carpet loot table should exist");
        let params = LootContextParameters {
            tool: None,
            killed_by_player: Some(false),
            ..Default::default()
        };
        let items = crate::world::loot::generate_loot_with_context(loot_table, 12345, &params);
        assert_eq!(items.len(), 1, "Carpet should drop exactly 1 item");
        assert_eq!(items[0].item.id, pumpkin_data::item::Item::RED_CARPET.id);
        assert_eq!(items[0].item_count, 1);
    }
}

