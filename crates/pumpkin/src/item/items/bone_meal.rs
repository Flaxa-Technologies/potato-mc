use std::any::Any;

use crate::entity::player::Player;
use crate::item::{ItemBehaviour, ItemMetadata};
use crate::server::Server;
use pumpkin_data::item::Item;
use pumpkin_data::item_stack::ItemStack;
use pumpkin_data::world::WorldEvent;
use pumpkin_data::{Block, BlockDirection};
use pumpkin_util::math::position::BlockPos;
use pumpkin_util::math::vector3::Vector3;

pub struct BoneMealItem;

impl ItemMetadata for BoneMealItem {
    fn ids() -> Box<[u16]> {
        Box::new([Item::BONE_MEAL.id])
    }
}

impl ItemBehaviour for BoneMealItem {
    #[allow(clippy::too_many_lines)]
    fn use_on_block(
        &self,
        item: &mut ItemStack,
        player: &Player,
        location: BlockPos,
        _face: BlockDirection,
        _cursor_pos: Vector3<f32>,
        block: &Block,
        server: &Server,
    ) {
        let world = player.world();
        let state_id = world.get_block_state_id(&location);
        if server
            .block_registry
            .bone_meal(block, &world, &location, state_id)
        {
            // WorldEvent 1505 (ParticlesAndSoundPlantGrowth) handles both the
            // green growth particles AND the bone-meal use sound on all clients.
            world.sync_world_event(WorldEvent::ParticlesAndSoundPlantGrowth, location, 15);
            player.swing_hand(pumpkin_util::Hand::Right, true);
            item.decrement_unless_creative(player.gamemode.load(), 1);
        }
    }


    fn as_any(&self) -> &dyn Any {
        self
    }
}
