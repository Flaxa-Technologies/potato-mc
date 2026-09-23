use std::sync::Arc;

use pumpkin_data::block_properties::BedPart;
use pumpkin_data::sound::{Sound, SoundCategory};
use pumpkin_data::translation;
use pumpkin_data::{Block, BlockId, BlockStateId};
use pumpkin_util::GameMode;
use pumpkin_util::math::position::BlockPos;
use pumpkin_world::world::BlockFlags;

use crate::block::bounce_entity_after_fall;
use crate::block::registry::BlockActionResult;
use crate::block::{
    BlockBehaviour, BlockMetadata, BrokenArgs, CanPlaceAtArgs, NormalUseArgs, OnLandedUponArgs,
    OnPlaceArgs, PlacedArgs, PlayerPlacedArgs, UpdateEntityMovementAfterFallOnArgs,
};
use crate::entity::EntityBase;
use crate::world::World;

type BedProperties = pumpkin_data::block_properties::WhiteBedLikeProperties;

/// Straw Bed block introduced in Minecraft 26.3.
/// Allows sleeping to skip the night, but DOES NOT set or update the player's personal respawn point.
/// Destroys itself with `BLOCK_STRAW_BED_BREAK_LEAVE` sound upon leaving or waking.
pub struct StrawBedBlock;

impl BlockMetadata for StrawBedBlock {
    fn ids() -> Box<[pumpkin_data::BlockId]> {
        [BlockId::STRAW_BED].into()
    }
}

impl StrawBedBlock {
    pub fn destroy_bed(world: &Arc<World>, pos: BlockPos) {
        world.play_sound(
            Sound::BlockStrawBedBreakLeave,
            SoundCategory::Blocks,
            &pos.to_f64(),
        );
        let state_id = world.get_block_state_id(&pos);
        if world.get_block(&pos).id == BlockId::STRAW_BED {
            let bed_props = BedProperties::from_state_id(state_id);
            let other_half_pos = if bed_props.part == BedPart::Head {
                pos.offset(bed_props.facing.opposite().to_offset())
            } else {
                pos.offset(bed_props.facing.to_offset())
            };
            world.set_block_state(&pos, Block::AIR.default_state.id, BlockFlags::NOTIFY_ALL);
            if world.get_block(&other_half_pos).id == BlockId::STRAW_BED {
                world.set_block_state(
                    &other_half_pos,
                    Block::AIR.default_state.id,
                    BlockFlags::NOTIFY_ALL,
                );
            }
        } else {
            world.set_block_state(&pos, Block::AIR.default_state.id, BlockFlags::NOTIFY_ALL);
        }
    }
}

impl BlockBehaviour for StrawBedBlock {
    fn can_place_at(&self, args: CanPlaceAtArgs<'_>) -> bool {
        if let Some(player) = args.player {
            let facing = player.get_entity().get_horizontal_facing();
            return args
                .block_accessor
                .get_block_state(args.position)
                .replaceable()
                && args
                    .block_accessor
                    .get_block_state(&args.position.offset(facing.to_offset()))
                    .replaceable();
        }
        false
    }

    fn on_landed_upon(&self, args: OnLandedUponArgs<'_>) {
        if let Some(living) = args.entity.get_living_entity() {
            living.handle_fall_damage(args.entity, args.fall_distance * 0.5, 1.0);
        }
    }

    fn update_entity_movement_after_fall_on(&self, args: UpdateEntityMovementAfterFallOnArgs<'_>) {
        bounce_entity_after_fall(args.entity, 0.66);
    }

    fn on_place(&self, args: OnPlaceArgs<'_>) -> BlockStateId {
        let mut bed_props = BedProperties::default(args.block);
        bed_props.facing = args.player.get_entity().get_horizontal_facing();
        bed_props.part = BedPart::Foot;
        bed_props.to_state_id(args.block)
    }

    fn placed(&self, args: PlacedArgs<'_>) {
        let mut bed_head_props = BedProperties::default(args.block);
        bed_head_props.facing = BedProperties::from_state_id(args.state_id).facing;
        bed_head_props.part = BedPart::Head;

        let bed_head_pos = args.position.offset(bed_head_props.facing.to_offset());
        args.world.set_block_state(
            &bed_head_pos,
            bed_head_props.to_state_id(args.block),
            BlockFlags::NOTIFY_ALL | BlockFlags::SKIP_BLOCK_ADDED_CALLBACK,
        );
    }

    fn player_placed(&self, args: PlayerPlacedArgs<'_>) {
        args.world.play_sound(
            Sound::BlockStrawBedPlace,
            SoundCategory::Blocks,
            &args.position.to_f64(),
        );
    }

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

        crate::block::blocks::bed::BedBlock::set_occupied(
            true,
            world,
            Block::from_id(BlockId::STRAW_BED),
            &bed_head_pos,
            world.get_block_state_id(&bed_head_pos),
        );

        player.sleep(bed_head_pos);
        // Straw bed sleep custom statistic (does NOT update player's personal respawn point)
        player.increment_stat(
            pumpkin_data::statistic::StatisticCategory::Custom,
            pumpkin_data::statistic::CustomStatistic::SleepInStrawBed as i32,
            1,
        );

        BlockActionResult::SuccessServer
    }

    fn broken(&self, args: BrokenArgs<'_>) {
        let bed_props = BedProperties::from_state_id(args.state.id);
        let other_half_pos = if bed_props.part == BedPart::Head {
            args.position
                .offset(bed_props.facing.opposite().to_offset())
        } else {
            args.position.offset(bed_props.facing.to_offset())
        };
        let other_half_block = args.world.get_block(&other_half_pos);
        if other_half_block.id == BlockId::STRAW_BED {
            let other_half_state = args.world.get_block_state_id(&other_half_pos);
            let other_half_props = BedProperties::from_state_id(other_half_state);
            if other_half_props.part != bed_props.part && other_half_props.facing == bed_props.facing {
                args.world.break_block(
                    &other_half_pos,
                    None,
                    BlockFlags::NOTIFY_ALL | BlockFlags::SKIP_DROPS,
                );
            }
        }
    }
}
