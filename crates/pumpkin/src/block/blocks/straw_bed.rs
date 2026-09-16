use std::sync::Arc;

use pumpkin_data::block_properties::BedPart;
use pumpkin_data::sound::{Sound, SoundCategory};
use pumpkin_data::translation;
use pumpkin_data::Block;
use pumpkin_util::GameMode;
use pumpkin_util::math::position::BlockPos;
use pumpkin_world::world::BlockFlags;

use crate::block::registry::BlockActionResult;
use crate::block::{BlockBehaviour, BlockMetadata, NormalUseArgs};
use crate::world::World;

type BedProperties = pumpkin_data::block_properties::WhiteBedLikeProperties;

/// Straw Bed block introduced in Minecraft 26.3.
/// Allows sleeping to skip the night, but DOES NOT set or update the player's personal respawn point.
/// Destroys itself with `BLOCK_STRAW_BED_BREAK_LEAVE` sound upon leaving or waking.
pub struct StrawBedBlock;

impl BlockMetadata for StrawBedBlock {
    fn ids() -> Box<[pumpkin_data::BlockId]> {
        [].into()
    }
}

impl StrawBedBlock {
    pub fn destroy_bed(world: &Arc<World>, pos: BlockPos) {
        world.play_sound(
            Sound::BlockStrawBedBreakLeave,
            SoundCategory::Blocks,
            &pos.to_f64(),
        );
        world.set_block_state(&pos, Block::AIR.default_state.id, BlockFlags::NOTIFY_ALL);
    }
}

impl BlockBehaviour for StrawBedBlock {
    fn normal_use(&self, args: NormalUseArgs<'_>) -> BlockActionResult {
        let player = args.player;
        let world = args.world;
        let position = args.position;

        if player.gamemode.load() == GameMode::Spectator {
            return BlockActionResult::Pass;
        }

        let state_id = world.get_block_state_id(position);
        let bed_props = BedProperties::from_state_id(state_id);

        let (bed_head_pos, bed_foot_pos) = if bed_props.part == BedPart::Head {
            (
                *position,
                position.offset(bed_props.facing.opposite().to_offset()),
            )
        } else {
            (position.offset(bed_props.facing.to_offset()), *position)
        };

        // Explode if dimension bed rule explodes (e.g. Nether/End)
        if world.dimension.bed_rule.explodes {
            world.break_block(&bed_head_pos, None, BlockFlags::SKIP_DROPS);
            world.break_block(&bed_foot_pos, None, BlockFlags::SKIP_DROPS);

            world.explode(
                bed_head_pos.to_centered_f64(),
                5.0,
                crate::world::ExplosionInteraction::Block,
            );

            return BlockActionResult::SuccessServer;
        }

        if bed_props.occupied {
            player.send_system_message_raw(
                &pumpkin_macros::translate_cross!(
                    translation::java::BLOCK_MINECRAFT_BED_OCCUPIED,
                    translation::bedrock::TILE_BED_OCCUPIED
                ),
                true,
            );
            return BlockActionResult::SuccessServer;
        }

        let is_dark = world.is_dark_outside();
        let can_sleep = world.dimension.bed_rule.can_sleep(is_dark);
        if !can_sleep {
            player.send_system_message_raw(
                &pumpkin_macros::translate_cross!(
                    translation::java::BLOCK_MINECRAFT_BED_NO_SLEEP,
                    translation::bedrock::TILE_BED_NOSLEEP
                ),
                true,
            );
            return BlockActionResult::SuccessServer;
        }

        player.sleep(bed_head_pos);
        // Straw bed sleep custom statistic (does NOT update player's personal respawn point)
        player.increment_stat(
            pumpkin_data::statistic::StatisticCategory::Custom,
            pumpkin_data::statistic::CustomStatistic::SleepInStrawBed as i32,
            1,
        );

        BlockActionResult::SuccessServer
    }
}
