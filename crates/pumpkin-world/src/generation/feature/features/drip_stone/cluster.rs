use pumpkin_data::tag;
use pumpkin_data::{Block, BlockDirection, BlockId};
use pumpkin_util::math::position::BlockPos;
use pumpkin_util::random::{RandomGenerator, RandomImpl};

use crate::generation::proto_chunk::GenerationCache;

pub struct DripstoneClusterFeature;

impl DripstoneClusterFeature {
    #[allow(clippy::unused_self)]
    pub fn generate<T: GenerationCache>(
        &self,
        chunk: &mut T,
        random: &mut RandomGenerator,
        pos: BlockPos,
    ) -> bool {
        if !super::is_empty_or_water(chunk, pos) {
            return false;
        }

        let height = random.next_inbetween_i32(3, 6);
        let wetness = (0.1f32 + (random.next_gaussian() as f32) * 0.3f32).clamp(0.1f32, 0.9f32);
        let density = random.next_inbetween_f32(0.3f32, 0.7f32);
        let x_radius = random.next_inbetween_i32(2, 8);
        let z_radius = random.next_inbetween_i32(2, 8);

        for dx in -x_radius..=x_radius {
            for dz in -z_radius..=z_radius {
                let chance_of_stalagmite_or_stalactite =
                    get_chance_of_stalagmite_or_stalactite(x_radius, z_radius, dx, dz);
                let col_pos = BlockPos::new(pos.0.x + dx, pos.0.y, pos.0.z + dz);
                place_column(
                    chunk,
                    random,
                    col_pos,
                    dx,
                    dz,
                    wetness,
                    chance_of_stalagmite_or_stalactite,
                    height,
                    density,
                );
            }
        }

        true
    }
}

#[allow(clippy::too_many_arguments)]
fn place_column<T: GenerationCache>(
    chunk: &mut T,
    random: &mut RandomGenerator,
    pos: BlockPos,
    dx: i32,
    dz: i32,
    chance_of_water: f32,
    chance_of_stalagmite_or_stalactite: f64,
    cluster_height: i32,
    density: f32,
) {
    if !super::is_empty_or_water(chunk, pos) {
        return;
    }

    let search_range = 12;
    let ceiling = scan_direction(chunk, search_range, pos, BlockDirection::Up);
    let base_floor = scan_direction(chunk, search_range, pos, BlockDirection::Down);

    if ceiling.is_none() && base_floor.is_none() {
        return;
    }

    let want_pool = random.next_f32() < chance_of_water;
    let floor = if want_pool
        && base_floor.is_some()
        && can_place_pool(chunk, BlockPos::new(pos.0.x, base_floor.unwrap(), pos.0.z))
    {
        let base_floor_y = base_floor.unwrap();
        chunk.set_block_state(
            &BlockPos::new(pos.0.x, base_floor_y, pos.0.z).0,
            Block::WATER.default_state,
        );
        Some(base_floor_y - 1)
    } else {
        base_floor
    };

    let want_stalactite = (random.next_f64()) < chance_of_stalagmite_or_stalactite;
    let stalactite_height = if let Some(ceiling_y) = ceiling {
        let ceiling_pos = BlockPos::new(pos.0.x, ceiling_y, pos.0.z);
        if want_stalactite && !is_lava(chunk, ceiling_pos) {
            let ceiling_thickness = random.next_inbetween_i32(2, 4);
            replace_blocks_with_base_blocks(
                chunk,
                ceiling_pos,
                ceiling_thickness,
                BlockDirection::Up,
            );
            let max_height_for_this_column = match floor {
                Some(floor_y) => cluster_height.min(ceiling_y - floor_y),
                None => cluster_height,
            };
            get_speleothem_height(random, dx, dz, density, max_height_for_this_column)
        } else {
            0
        }
    } else {
        0
    };

    let want_stalagmite = (random.next_f64()) < chance_of_stalagmite_or_stalactite;
    let stalagmite_height = if let Some(floor_y) = floor {
        let floor_pos = BlockPos::new(pos.0.x, floor_y, pos.0.z);
        if want_stalagmite && !is_lava(chunk, floor_pos) {
            let floor_thickness = random.next_inbetween_i32(2, 4);
            replace_blocks_with_base_blocks(
                chunk,
                floor_pos,
                floor_thickness,
                BlockDirection::Down,
            );
            if ceiling.is_some() {
                (stalactite_height + random.next_inbetween_i32(-1, 1)).max(0)
            } else {
                get_speleothem_height(random, dx, dz, density, cluster_height)
            }
        } else {
            0
        }
    } else {
        0
    };

    let (actual_stalactite_height, actual_stalagmite_height) =
        if let (Some(ceiling_y), Some(floor_y)) = (ceiling, floor) {
            if ceiling_y - stalactite_height <= floor_y + stalagmite_height {
                let lowest_stalactite_bottom = (ceiling_y - stalactite_height).max(floor_y + 1);
                let highest_stalagmite_top = (floor_y + stalagmite_height).min(ceiling_y - 1);
                let actual_stalactite_bottom = random
                    .next_inbetween_i32(lowest_stalactite_bottom, highest_stalagmite_top + 1);
                let actual_stalagmite_top = actual_stalactite_bottom - 1;
                let actual_c = ceiling_y - actual_stalactite_bottom;
                let actual_s = actual_stalagmite_top - floor_y;
                (actual_c, actual_s)
            } else {
                (stalactite_height, stalagmite_height)
            }
        } else {
            (stalactite_height, stalagmite_height)
        };

    let column_height = match (ceiling, floor) {
        (Some(c), Some(f)) => Some(c - f - 1),
        _ => None,
    };
    let merge_tips = random.next_bool()
        && actual_stalactite_height > 0
        && actual_stalagmite_height > 0
        && column_height.is_some()
        && actual_stalactite_height + actual_stalagmite_height == column_height.unwrap();

    if let Some(ceiling_y) = ceiling {
        let root_pos = BlockPos::new(pos.0.x, ceiling_y, pos.0.z);
        if super::can_replace(GenerationCache::get_block_state(chunk, &root_pos.0).to_block_id()) {
            super::grow_pointed_dripstone(
                chunk,
                BlockPos::new(pos.0.x, ceiling_y - 1, pos.0.z),
                BlockDirection::Down,
                actual_stalactite_height,
                merge_tips,
            );
        }
    }

    if let Some(floor_y) = floor {
        let root_pos = BlockPos::new(pos.0.x, floor_y, pos.0.z);
        if super::can_replace(GenerationCache::get_block_state(chunk, &root_pos.0).to_block_id()) {
            super::grow_pointed_dripstone(
                chunk,
                BlockPos::new(pos.0.x, floor_y + 1, pos.0.z),
                BlockDirection::Up,
                actual_stalagmite_height,
                merge_tips,
            );
        }
    }
}

fn scan_direction<T: GenerationCache>(
    chunk: &T,
    search_range: i32,
    pos: BlockPos,
    direction: BlockDirection,
) -> Option<i32> {
    let mut cur = pos;
    for _ in 1..search_range {
        if !super::is_empty_or_water(chunk, cur) {
            break;
        }
        cur = cur.offset(direction.to_offset());
    }
    if !super::is_empty_or_water(chunk, cur) {
        Some(cur.0.y)
    } else {
        None
    }
}

fn can_place_pool<T: GenerationCache>(chunk: &T, pos: BlockPos) -> bool {
    let block = GenerationCache::get_block_state(chunk, &pos.0).to_block_id();
    if block != BlockId::WATER
        && block != BlockId::DRIPSTONE_BLOCK
        && block != BlockId::POINTED_DRIPSTONE
    {
        let above = GenerationCache::get_block_state(chunk, &pos.up().0).to_block_id();
        if above == BlockId::WATER {
            return false;
        }
        for dir in [
            BlockDirection::North,
            BlockDirection::South,
            BlockDirection::East,
            BlockDirection::West,
        ] {
            if !can_be_adjacent_to_water(chunk, pos.offset(dir.to_offset())) {
                return false;
            }
        }
        can_be_adjacent_to_water(chunk, pos.down())
    } else {
        false
    }
}

fn can_be_adjacent_to_water<T: GenerationCache>(chunk: &T, pos: BlockPos) -> bool {
    let block = GenerationCache::get_block_state(chunk, &pos.0).to_block_id();
    block.has_tag(tag::Block::MINECRAFT_BASE_STONE_OVERWORLD) || block == BlockId::WATER
}

fn is_lava<T: GenerationCache>(chunk: &T, pos: BlockPos) -> bool {
    GenerationCache::get_block_state(chunk, &pos.0).to_block_id() == BlockId::LAVA
}

fn replace_blocks_with_base_blocks<T: GenerationCache>(
    chunk: &mut T,
    first_pos: BlockPos,
    max_count: i32,
    direction: BlockDirection,
) {
    let mut cur = first_pos;
    let bottom_y = chunk.bottom_y() as i32;
    let top_y = bottom_y + chunk.height() as i32;
    for _ in 0..max_count {
        if cur.0.y < bottom_y || cur.0.y >= top_y {
            return;
        }
        if !place_base_block_if_possible(chunk, cur) {
            return;
        }
        cur = cur.offset(direction.to_offset());
    }
}

fn place_base_block_if_possible<T: GenerationCache>(chunk: &mut T, pos: BlockPos) -> bool {
    let block = GenerationCache::get_block_state(chunk, &pos.0).to_block_id();
    if block.has_tag(tag::Block::MINECRAFT_DRIPSTONE_REPLACEABLE_BLOCKS) {
        chunk.set_block_state(&pos.0, Block::DRIPSTONE_BLOCK.default_state);
        true
    } else {
        false
    }
}

fn get_chance_of_stalagmite_or_stalactite(
    x_radius: i32,
    z_radius: i32,
    dx: i32,
    dz: i32,
) -> f64 {
    let x_dist_from_edge = x_radius - dx.abs();
    let z_dist_from_edge = z_radius - dz.abs();
    let dist_from_edge = x_dist_from_edge.min(z_dist_from_edge) as f64;
    if dist_from_edge <= 0.0 {
        0.1
    } else if dist_from_edge >= 3.0 {
        1.0
    } else {
        0.1 + (dist_from_edge / 3.0) * 0.9
    }
}

fn get_speleothem_height(
    random: &mut RandomGenerator,
    dx: i32,
    dz: i32,
    density: f32,
    max_height: i32,
) -> i32 {
    if random.next_f32() > density {
        return 0;
    }
    let dist_from_center = (dx.abs() + dz.abs()) as f32;
    let height_mean = if dist_from_center <= 0.0 {
        max_height as f32 / 2.0
    } else if dist_from_center >= 8.0 {
        0.0
    } else {
        (max_height as f32 / 2.0) * (1.0 - dist_from_center / 8.0)
    };
    let gaussian = random.next_gaussian() as f32;
    let sampled = (height_mean + gaussian * 3.0).clamp(0.0, max_height as f32);
    sampled as i32
}
