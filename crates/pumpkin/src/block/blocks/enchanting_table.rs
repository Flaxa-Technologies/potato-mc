use std::sync::Arc;
use std::sync::Mutex;

use crate::block::entities::enchanting_table::EnchantingTableBlockEntity;
use crate::block::registry::BlockActionResult;
use crate::block::{
    BlockBehaviour, GetScreenHandlerFactoryArgs, NormalUseArgs, PathComputationType, PlacedArgs,
};
use pumpkin_data::tag::Taggable;
use pumpkin_data::{BlockState, translation};
use pumpkin_inventory::enchanting::enchanting_screen_handler::EnchantingTableScreenHandler;
use pumpkin_inventory::player::player_inventory::PlayerInventory;
use pumpkin_inventory::screen_handler::{
    InventoryPlayer, ScreenHandlerFactory, SharedScreenHandler,
};
use pumpkin_macros::pumpkin_block;
use pumpkin_util::math::position::BlockPos;
use pumpkin_util::text::TextComponent;
use pumpkin_world::inventory::{Inventory, SimpleInventory};

#[pumpkin_block("minecraft:enchanting_table")]
pub struct EnchantingTableBlock;

impl BlockBehaviour for EnchantingTableBlock {
    fn placed(&self, args: PlacedArgs<'_>) {
        let entity = EnchantingTableBlockEntity::new(*args.position);
        args.world.add_block_entity(Arc::new(entity));
    }

    fn normal_use(&self, args: NormalUseArgs<'_>) -> BlockActionResult {
        if let Some(factory) = self.get_screen_handler_factory(GetScreenHandlerFactoryArgs {
            server: args.server,
            world: args.world,
            block: args.block,
            position: args.position,
            player: args.player,
        }) {
            args.player
                .open_handled_screen(factory.as_ref(), Some(*args.position));
        }
        BlockActionResult::Success
    }

    fn get_screen_handler_factory(
        &self,
        args: GetScreenHandlerFactoryArgs<'_>,
    ) -> Option<Box<dyn ScreenHandlerFactory>> {
        let mut bookshelf_count = 0;

        for off_x in -2..=2i32 {
            for off_y in 0..=1i32 {
                for off_z in -2..=2i32 {
                    if (off_x.abs() == 2 || off_z.abs() == 2)
                        && Self::is_valid_bookshelf(args.world, args.position, off_x, off_y, off_z)
                    {
                        bookshelf_count += 1;
                    }
                }
            }
        }
        let bookshelf_count = bookshelf_count.min(15);

        let seed = args.player.enchantment_seed();
        Some(Box::new(EnchantingTableScreenFactory {
            bookshelf_count,
            seed,
        }))
    }

    fn is_pathfindable(&self, _state: &BlockState, _computation_type: PathComputationType) -> bool {
        false
    }
}

impl EnchantingTableBlock {
    fn is_valid_bookshelf(
        world: &Arc<crate::world::World>,
        table_pos: &BlockPos,
        off_x: i32,
        off_y: i32,
        off_z: i32,
    ) -> bool {
        let shelf_pos = table_pos.add(off_x, off_y, off_z);
        let shelf_state = world.get_block_state(&shelf_pos);
        let shelf_block = pumpkin_data::Block::from_state_id(shelf_state.id);
        if !shelf_block.has_tag(&pumpkin_data::tag::Block::MINECRAFT_ENCHANTMENT_POWER_PROVIDER) {
            return false;
        }

        let transmitter_pos = table_pos.add(off_x / 2, off_y, off_z / 2);
        let transmitter_state = world.get_block_state(&transmitter_pos);
        let transmitter_block = pumpkin_data::Block::from_state_id(transmitter_state.id);
        transmitter_block.has_tag(&pumpkin_data::tag::Block::MINECRAFT_ENCHANTMENT_POWER_TRANSMITTER)
    }
}

struct EnchantingTableScreenFactory {
    bookshelf_count: i32,
    seed: i32,
}

impl ScreenHandlerFactory for EnchantingTableScreenFactory {
    fn create_screen_handler(
        &self,
        sync_id: u8,
        player_inventory: &Arc<PlayerInventory>,
        _player: &dyn InventoryPlayer,
    ) -> Option<SharedScreenHandler> {
        let inventory: Arc<dyn Inventory> = Arc::new(SimpleInventory::new(2));
        let handler = EnchantingTableScreenHandler::new(
            sync_id,
            player_inventory,
            &inventory,
            self.seed,
            self.bookshelf_count,
        );
        let screen_handler_arc = Arc::new(Mutex::new(handler));
        Some(screen_handler_arc as SharedScreenHandler)
    }

    fn get_display_name(&self) -> TextComponent {
        pumpkin_macros::translate_cross!(
            translation::java::CONTAINER_ENCHANT,
            translation::bedrock::CONTAINER_ENCHANT
        )
    }
}
