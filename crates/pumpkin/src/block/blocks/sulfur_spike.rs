use std::sync::Arc;

use crate::{
    block::{
        BlockBehaviour, BrokenArgs, CanPlaceAtArgs, GetStateForNeighborUpdateArgs, OnPlaceArgs,
        PathComputationType, PlacedArgs, RandomTickArgs,
    },
    entity::player::Player,
    world::World,
};
use pumpkin_data::{
    Block, BlockDirection, BlockState, BlockStateId,
    block_properties::{PointedDripstoneLikeProperties, SpeleothemThickness, VerticalDirection},
};
use pumpkin_macros::pumpkin_block;
use pumpkin_util::math::position::BlockPos;
use pumpkin_util::math::vector3::Vector3;
use pumpkin_world::world::{BlockAccessor, BlockFlags};

#[pumpkin_block("minecraft:sulfur_spike")]
pub struct SulfurSpikeBlock;

impl BlockBehaviour for SulfurSpikeBlock {
    fn can_place_at(&self, args: CanPlaceAtArgs<'_>) -> bool {
        can_place_at_pos(
            args.block_accessor,
            args.position,
            args.direction,
            args.player,
        )
    }

    fn on_place(&self, args: OnPlaceArgs<'_>) -> BlockStateId {
        let mut spike_props = PointedDripstoneLikeProperties::default(args.block);
        spike_props.waterlogged = args.replacing.water_source();
        let Some(support_block_ver_dir) = get_support_block_vertical_direction(
            args.world,
            args.position,
            Some(args.direction),
            Some(args.player),
        ) else {
            return Block::AIR.default_state.id;
        };

        spike_props.vertical_direction = flip_dir(support_block_ver_dir);
        spike_props.to_state_id(&Block::SULFUR_SPIKE)
    }

    fn placed(&self, args: PlacedArgs<'_>) {
        let (len, vertical_dir) = get_stalagmite_or_stalactite_len_and_dir_from_tip_pos(
            args.world,
            args.position,
            args.state_id,
        );
        match vertical_dir {
            VerticalDirection::Up => {
                update_stalagmite(args.world, len, args.position);
            }
            VerticalDirection::Down => {
                update_stalactite(args.world, len, args.position);
            }
        }
    }

    fn broken(&self, args: BrokenArgs<'_>) {
        let broken_spike_props = PointedDripstoneLikeProperties::from_state_id(args.state.id);
        let new_tip_pos = match broken_spike_props.vertical_direction {
            VerticalDirection::Up => args.position.down(),
            VerticalDirection::Down => args.position.up(),
        };

        let (len, vertical_dir) = get_stalagmite_or_stalactite_len_and_dir_from_tip_pos(
            args.world,
            &new_tip_pos,
            args.state.id,
        );
        match vertical_dir {
            VerticalDirection::Up => {
                update_stalagmite(args.world, len, &new_tip_pos);
            }
            VerticalDirection::Down => {
                update_stalactite(args.world, len, &new_tip_pos);
            }
        }
    }

    fn get_state_for_neighbor_update(
        &self,
        args: GetStateForNeighborUpdateArgs<'_>,
    ) -> BlockStateId {
        if !can_place_at_pos(args.world, args.position, None, None) {
            return Block::AIR.default_state.id;
        }
        let mut spike_props = PointedDripstoneLikeProperties::from_state_id(args.state_id);
        if spike_props.thickness != SpeleothemThickness::TipMerge {
            return args.state_id;
        }
        match spike_props.vertical_direction {
            VerticalDirection::Up => {
                let block_above = args.world.get_block(&args.position.up());
                if block_above != &Block::SULFUR_SPIKE {
                    spike_props.thickness = SpeleothemThickness::Tip;
                    return spike_props.to_state_id(args.block);
                }
            }
            VerticalDirection::Down => {
                let block_below = args.world.get_block(&args.position.down());
                if block_below != &Block::SULFUR_SPIKE {
                    spike_props.thickness = SpeleothemThickness::Tip;
                    return spike_props.to_state_id(args.block);
                }
            }
        }
        args.state_id
    }

    fn random_tick(&self, args: RandomTickArgs<'_>) {
        // Growth when hanging off a sulfur block (max length 2 per SulfurSpikeBlock.java)
        let props = PointedDripstoneLikeProperties::from_state_id(
            args.world.get_block_state(args.position).id,
        );
        if props.vertical_direction == VerticalDirection::Down {
            let (len, _) = get_stalagmite_or_stalactite_len_and_dir_from_tip_pos(
                args.world,
                args.position,
                args.world.get_block_state(args.position).id,
            );
            if len < 2 {
                // Check root block is sulfur
                let root_pos = args.position.up_height(len as i32);
                if args.world.get_block(&root_pos) == &Block::SULFUR {
                    let below_tip = args.position.down();
                    if args.world.get_block_state(&below_tip).is_air() {
                        let mut new_props = PointedDripstoneLikeProperties::default(&Block::SULFUR_SPIKE);
                        new_props.vertical_direction = VerticalDirection::Down;
                        new_props.thickness = SpeleothemThickness::Tip;
                        args.world.set_block_state(
                            &below_tip,
                            new_props.to_state_id(&Block::SULFUR_SPIKE),
                            BlockFlags::NOTIFY_ALL,
                        );
                        update_stalactite(args.world, len + 1, &below_tip);
                    }
                }
            }
        }
    }

    fn is_pathfindable(&self, _state: &BlockState, _computation_type: PathComputationType) -> bool {
        false
    }
}

fn update_stalagmite(world: &Arc<World>, len: u8, tip_pos: &BlockPos) {
    let block_above = world.get_block(&tip_pos.up());
    if block_above == &Block::SULFUR_SPIKE {
        modify_spike_thickness_to(world, tip_pos, SpeleothemThickness::TipMerge);
        modify_spike_thickness_to(world, &tip_pos.up(), SpeleothemThickness::TipMerge);
    } else {
        modify_spike_thickness_to(world, tip_pos, SpeleothemThickness::Tip);
    }
    match len {
        2 => {
            modify_spike_thickness_to(world, &tip_pos.down_height(1), SpeleothemThickness::Frustum);
        }
        3 => {
            modify_spike_thickness_to(world, &tip_pos.down_height(1), SpeleothemThickness::Frustum);
            modify_spike_thickness_to(world, &tip_pos.down_height(2), SpeleothemThickness::Base);
        }
        4 => {
            modify_spike_thickness_to(world, &tip_pos.down_height(1), SpeleothemThickness::Frustum);
            modify_spike_thickness_to(world, &tip_pos.down_height(2), SpeleothemThickness::Middle);
            modify_spike_thickness_to(world, &tip_pos.down_height(3), SpeleothemThickness::Base);
        }
        _ => {}
    }
}

fn update_stalactite(world: &Arc<World>, len: u8, tip_pos: &BlockPos) {
    let block_below = world.get_block(&tip_pos.down());
    if block_below == &Block::SULFUR_SPIKE {
        modify_spike_thickness_to(world, tip_pos, SpeleothemThickness::TipMerge);
        modify_spike_thickness_to(world, &tip_pos.down(), SpeleothemThickness::TipMerge);
    } else {
        modify_spike_thickness_to(world, tip_pos, SpeleothemThickness::Tip);
    }
    match len {
        2 => {
            modify_spike_thickness_to(world, &tip_pos.up_height(1), SpeleothemThickness::Frustum);
        }
        3 => {
            modify_spike_thickness_to(world, &tip_pos.up_height(1), SpeleothemThickness::Frustum);
            modify_spike_thickness_to(world, &tip_pos.up_height(2), SpeleothemThickness::Base);
        }
        4 => {
            modify_spike_thickness_to(world, &tip_pos.up_height(1), SpeleothemThickness::Frustum);
            modify_spike_thickness_to(world, &tip_pos.up_height(2), SpeleothemThickness::Middle);
            modify_spike_thickness_to(world, &tip_pos.up_height(3), SpeleothemThickness::Base);
        }
        _ => {}
    }
}

fn modify_spike_thickness_to(world: &Arc<World>, pos: &BlockPos, thickness: SpeleothemThickness) {
    let state = world.get_block_state(pos);
    if state.id.to_block_id() == Block::SULFUR_SPIKE.id {
        let mut props = PointedDripstoneLikeProperties::from_state_id(state.id);
        props.thickness = thickness;
        world.set_block_state(pos, props.to_state_id(&Block::SULFUR_SPIKE), BlockFlags::NOTIFY_ALL);
    }
}

fn can_place_at_pos(
    world: &dyn BlockAccessor,
    pos: &BlockPos,
    placed_face: Option<BlockDirection>,
    player: Option<&Player>,
) -> bool {
    let Some(vertical_dir) =
        get_support_block_vertical_direction(world, pos, placed_face, player)
    else {
        return false;
    };
    let support_pos = match vertical_dir {
        VerticalDirection::Up => pos.down(),
        VerticalDirection::Down => pos.up(),
    };
    can_place_spike_on(world, &support_pos, vertical_dir)
}

fn can_place_spike_on(world: &dyn BlockAccessor, pos: &BlockPos, dir: VerticalDirection) -> bool {
    let block = world.get_block(pos);
    if block == &Block::SULFUR || block == &Block::SULFUR_SPIKE || block.default_state.is_full_cube() {
        if block == &Block::SULFUR_SPIKE {
            let props = PointedDripstoneLikeProperties::from_state_id(world.get_block_state(pos).id);
            return props.vertical_direction == flip_dir(dir);
        }
        return true;
    }
    false
}

const fn flip_dir(dir: VerticalDirection) -> VerticalDirection {
    match dir {
        VerticalDirection::Up => VerticalDirection::Down,
        VerticalDirection::Down => VerticalDirection::Up,
    }
}

fn get_support_block_vertical_direction(
    world: &dyn BlockAccessor,
    pos: &BlockPos,
    placed_face: Option<BlockDirection>,
    _player: Option<&Player>,
) -> Option<VerticalDirection> {
    if let Some(face) = placed_face {
        match face {
            BlockDirection::Up => {
                if can_place_spike_on(world, &pos.down(), VerticalDirection::Up) {
                    return Some(VerticalDirection::Up);
                }
            }
            BlockDirection::Down => {
                if can_place_spike_on(world, &pos.up(), VerticalDirection::Down) {
                    return Some(VerticalDirection::Down);
                }
            }
            _ => {}
        }
    }
    if can_place_spike_on(world, &pos.down(), VerticalDirection::Up) {
        Some(VerticalDirection::Up)
    } else if can_place_spike_on(world, &pos.up(), VerticalDirection::Down) {
        Some(VerticalDirection::Down)
    } else {
        None
    }
}

fn get_stalagmite_or_stalactite_len_and_dir_from_tip_pos(
    world: &World,
    tip_pos: &BlockPos,
    tip_state_id: BlockStateId,
) -> (u8, VerticalDirection) {
    let tip_props = PointedDripstoneLikeProperties::from_state_id(tip_state_id);
    let mut len = 1u8;
    let mut current_pos = *tip_pos;
    let offset = match tip_props.vertical_direction {
        VerticalDirection::Up => -1,
        VerticalDirection::Down => 1,
    };
    loop {
        current_pos = current_pos.offset(Vector3::new(0, offset, 0));
        let block = world.get_block(&current_pos);
        if block == &Block::SULFUR_SPIKE {
            let props = PointedDripstoneLikeProperties::from_state_id(
                world.get_block_state(&current_pos).id,
            );
            if props.vertical_direction == tip_props.vertical_direction {
                len += 1;
                continue;
            }
        }
        break;
    }
    (len, tip_props.vertical_direction)
}
