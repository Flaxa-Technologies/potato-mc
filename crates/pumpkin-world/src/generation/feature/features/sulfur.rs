use pumpkin_data::block_properties::{
    PointedDripstoneLikeProperties, PotentSulfurLikeProperties, PotentSulfurState,
    SpeleothemThickness, VerticalDirection,
};
use pumpkin_data::{Block, BlockDirection, BlockId, BlockState, Mirror, Rotation, tag};
use pumpkin_util::math::position::BlockPos;
use pumpkin_util::math::vector3::Vector3;
use pumpkin_util::random::{RandomGenerator, RandomImpl};

use crate::generation::block_state_provider::{BlockStateProvider, SimpleStateProvider};
use crate::generation::feature::features::lake::LakeFeature;
use crate::generation::proto_chunk::GenerationCache;
use crate::generation::structure::template::{BlockStateResolver, get_template};
use crate::world::WorldPortalExt;

pub struct SulfurSpringFeature;

impl SulfurSpringFeature {
    pub fn generate<T: GenerationCache>(
        &self,
        chunk: &mut T,
        random: &mut RandomGenerator,
        pos: BlockPos,
    ) -> bool {
        // Variant selection based on weights:
        // small: wt 200, med: wt 90, lrg: wt 20, xlrg: wt 5 (total 315)
        let roll = random.next_bounded_i32(315);
        let (tuff_count, spread, templates): (i32, i32, &[&str]) = if roll < 200 {
            (
                64,
                7,
                &[
                    "spring/sulfur_spring_small_1",
                    "spring/sulfur_spring_small_2",
                    "spring/sulfur_spring_small_3",
                    "spring/sulfur_spring_small_4",
                ],
            )
        } else if roll < 290 {
            (
                80,
                8,
                &[
                    "spring/sulfur_spring_medium_1",
                    "spring/sulfur_spring_medium_2",
                    "spring/sulfur_spring_medium_3",
                ],
            )
        } else if roll < 310 {
            (
                96,
                9,
                &[
                    "spring/sulfur_spring_large_1",
                    "spring/sulfur_spring_large_2",
                ],
            )
        } else {
            (
                128,
                10,
                &["spring/sulfur_spring_extra_large_1"],
            )
        };

        // 1. Place tuff ring around spring
        for _ in 0..tuff_count {
            let dx = random.next_inbetween_i32(-spread, spread);
            let dz = random.next_inbetween_i32(-spread, spread);
            let dy = random.next_inbetween_i32(-3, 3);
            let target_pos = pos.add(dx, dy, dz);

            let mut cur = target_pos;
            for _ in 0..4 {
                let state = GenerationCache::get_block_state(chunk, &cur.0).to_state();
                if state.is_solid() {
                    chunk.set_block_state(&cur.0, Block::TUFF.default_state);
                    break;
                }
                cur = cur.down();
            }
        }

        // 2. Place embedded structure template at Y - 7
        let template_name = templates[random.next_bounded_i32(templates.len() as i32) as usize];
        if let Some(template) = get_template(template_name) {
            let origin = Vector3::new(pos.0.x, pos.0.y - 7, pos.0.z);
            for block in &template.blocks {
                let world_pos = origin + block.pos;
                let palette_entry = &template.palette[block.state as usize];
                if palette_entry.name == "minecraft:structure_block"
                    || palette_entry.name == "minecraft:structure_void"
                {
                    continue;
                }
                if let Some(state) = BlockStateResolver::resolve(
                    palette_entry,
                    Rotation::None,
                    Mirror::None,
                ) {
                    chunk.set_block_state(&world_pos, state);
                }
            }
            return true;
        }

        false
    }
}

pub struct SulfurPoolFeature;

impl SulfurPoolFeature {
    pub fn generate<T: GenerationCache>(
        &self,
        block_registry: &dyn WorldPortalExt,
        chunk: &mut T,
        random: &mut RandomGenerator,
        pos: BlockPos,
    ) -> bool {
        let lake = LakeFeature {
            fluid: BlockStateProvider::Simple(SimpleStateProvider {
                state: Block::WATER.default_state,
            }),
            barrier: BlockStateProvider::Simple(SimpleStateProvider {
                state: Block::SULFUR.default_state,
            }),
        };

        if !lake.generate(block_registry, chunk, random, pos) {
            return false;
        }

        // Scan bottom floor of the lake for potent sulfur placement
        let origin = pos.add(-8, -4, -8);
        for xx in 0..16 {
            for zz in 0..16 {
                for yyx in 0..7 {
                    let floor_pos = origin.add(xx, yyx, zz);
                    let above_pos = origin.add(xx, yyx + 1, zz);

                    let floor_state =
                        GenerationCache::get_block_state(chunk, &floor_pos.0).to_state();
                    let above_id =
                        GenerationCache::get_block_state(chunk, &above_pos.0).to_block_id();

                    if floor_state.is_solid() && above_id == BlockId::WATER {
                        let mut props =
                            PotentSulfurLikeProperties::default(&Block::POTENT_SULFUR);
                        props.r#potent_sulfur_state = PotentSulfurState::Wet;
                        let state_id = props.to_state_id(&Block::POTENT_SULFUR);
                        chunk.set_block_state(&floor_pos.0, &BlockState::from_id(state_id));
                    }
                }
            }
        }

        true
    }
}

pub struct SulfurSpikeClusterFeature;

impl SulfurSpikeClusterFeature {
    pub fn generate<T: GenerationCache>(
        &self,
        chunk: &mut T,
        random: &mut RandomGenerator,
        pos: BlockPos,
    ) -> bool {
        if !is_empty_or_water(chunk, pos) {
            return false;
        }

        let height = random.next_inbetween_i32(1, 4);
        let density = random.next_inbetween_f32(0.3, 0.7);
        let x_radius = random.next_inbetween_i32(2, 8);
        let z_radius = random.next_inbetween_i32(2, 8);

        for dx in -x_radius..=x_radius {
            for dz in -z_radius..=z_radius {
                let dist = ((dx * dx) as f64 / (x_radius * x_radius) as f64
                    + (dz * dz) as f64 / (z_radius * z_radius) as f64)
                    .sqrt();
                if dist > 1.0 {
                    continue;
                }
                let chance = 1.0 - dist * 0.9;
                if random.next_f64() > chance {
                    continue;
                }

                let col_pos = BlockPos::new(pos.0.x + dx, pos.0.y, pos.0.z + dz);
                let ceiling = scan_direction(chunk, 12, col_pos, BlockDirection::Up);
                let floor = scan_direction(chunk, 12, col_pos, BlockDirection::Down);

                if ceiling.is_none() && floor.is_none() {
                    continue;
                }

                let column_height = match (ceiling, floor) {
                    (Some(c), Some(f)) => Some(c - f - 1),
                    _ => None,
                };

                let mut actual_stalactite_height = 0;
                let mut actual_stalagmite_height = 0;

                if let Some(ch) = column_height {
                    if ch < 1 {
                        continue;
                    }
                    if random.next_f32() < density {
                        actual_stalactite_height = (random.next_inbetween_i32(1, height.min(ch))).min(2);
                    }
                    if random.next_f32() < density {
                        actual_stalagmite_height = (random.next_inbetween_i32(1, height.min(ch - actual_stalactite_height))).min(2);
                    }
                } else {
                    if ceiling.is_some() && random.next_f32() < density {
                        actual_stalactite_height = random.next_inbetween_i32(1, height).min(2);
                    }
                    if floor.is_some() && random.next_f32() < density {
                        actual_stalagmite_height = random.next_inbetween_i32(1, height).min(2);
                    }
                }

                if let Some(ceiling_y) = ceiling {
                    let root_pos = BlockPos::new(col_pos.0.x, ceiling_y, col_pos.0.z);
                    if can_replace_sulfur_base(
                        GenerationCache::get_block_state(chunk, &root_pos.0).to_block_id(),
                    ) {
                        gen_sulfur_base(chunk, root_pos);
                        if actual_stalactite_height > 0 {
                            grow_pointed_sulfur_spike(
                                chunk,
                                BlockPos::new(col_pos.0.x, ceiling_y - 1, col_pos.0.z),
                                BlockDirection::Down,
                                actual_stalactite_height,
                                false,
                            );
                        }
                    }
                }

                if let Some(floor_y) = floor {
                    let root_pos = BlockPos::new(col_pos.0.x, floor_y, col_pos.0.z);
                    if can_replace_sulfur_base(
                        GenerationCache::get_block_state(chunk, &root_pos.0).to_block_id(),
                    ) {
                        gen_sulfur_base(chunk, root_pos);
                        if actual_stalagmite_height > 0 {
                            grow_pointed_sulfur_spike(
                                chunk,
                                BlockPos::new(col_pos.0.x, floor_y + 1, col_pos.0.z),
                                BlockDirection::Up,
                                actual_stalagmite_height,
                                false,
                            );
                        }
                    }
                }
            }
        }

        true
    }
}

pub struct SulfurSpikeFeature;

impl SulfurSpikeFeature {
    pub fn generate<T: GenerationCache>(
        &self,
        chunk: &mut T,
        random: &mut RandomGenerator,
        pos: BlockPos,
    ) -> bool {
        if !is_empty_or_water(chunk, pos) {
            return false;
        }

        let up_replaceable = can_replace_sulfur_base(
            GenerationCache::get_block_state(chunk, &pos.up().0).to_block_id(),
        );
        let down_replaceable = can_replace_sulfur_base(
            GenerationCache::get_block_state(chunk, &pos.down().0).to_block_id(),
        );

        let dir = match (up_replaceable, down_replaceable) {
            (true, true) => {
                if random.next_bool() {
                    BlockDirection::Down
                } else {
                    BlockDirection::Up
                }
            }
            (true, false) => BlockDirection::Down,
            (false, true) => BlockDirection::Up,
            (false, false) => return false,
        };

        let root_pos = pos.offset(dir.opposite().to_offset());
        gen_sulfur_base(chunk, root_pos);

        let next_pos = pos.offset(dir.to_offset());
        let height = if random.next_f32() < 0.2 && is_empty_or_water(chunk, next_pos) {
            2
        } else {
            1
        };

        grow_pointed_sulfur_spike(chunk, pos, dir, height, false);
        true
    }
}

fn grow_pointed_sulfur_spike<T: GenerationCache>(
    chunk: &mut T,
    start_pos: BlockPos,
    tip_direction: BlockDirection,
    height: i32,
    merged_tip: bool,
) {
    let mut cur = start_pos;
    let vert_dir = match tip_direction {
        BlockDirection::Down => VerticalDirection::Down,
        _ => VerticalDirection::Up,
    };

    let mut thicknesses = Vec::with_capacity(height as usize);
    if height >= 3 {
        thicknesses.push(SpeleothemThickness::Base);
        for _ in 0..(height - 3) {
            thicknesses.push(SpeleothemThickness::Middle);
        }
    }
    if height >= 2 {
        thicknesses.push(SpeleothemThickness::Frustum);
    }
    if height >= 1 {
        thicknesses.push(if merged_tip {
            SpeleothemThickness::TipMerge
        } else {
            SpeleothemThickness::Tip
        });
    }

    for thickness in thicknesses {
        let is_water =
            GenerationCache::get_block_state(chunk, &cur.0).to_block_id() == BlockId::WATER;
        let mut props = PointedDripstoneLikeProperties::default(&Block::SULFUR_SPIKE);
        props.thickness = thickness;
        props.vertical_direction = vert_dir;
        props.waterlogged = is_water;
        let state_id = props.to_state_id(&Block::SULFUR_SPIKE);
        chunk.set_block_state(&cur.0, &BlockState::from_id(state_id));
        cur = cur.offset(tip_direction.to_offset());
    }
}

fn is_empty_or_water<T: GenerationCache>(chunk: &T, pos: BlockPos) -> bool {
    let block = GenerationCache::get_block_state(chunk, &pos.0).to_block_id();
    block == BlockId::AIR
        || block == BlockId::CAVE_AIR
        || block == BlockId::VOID_AIR
        || block == BlockId::WATER
}

fn can_replace_sulfur_base(id: BlockId) -> bool {
    id == BlockId::SULFUR || id.has_tag(tag::Block::MINECRAFT_SULFUR_SPIKE_REPLACEABLE_BLOCKS)
}

fn gen_sulfur_base<T: GenerationCache>(chunk: &mut T, pos: BlockPos) -> bool {
    let block = GenerationCache::get_block_state(chunk, &pos.0).to_block_id();
    if block.has_tag(tag::Block::MINECRAFT_SULFUR_SPIKE_REPLACEABLE_BLOCKS) {
        chunk.set_block_state(&pos.0, Block::SULFUR.default_state);
        return true;
    }
    block == BlockId::SULFUR
}

fn scan_direction<T: GenerationCache>(
    chunk: &T,
    search_range: i32,
    pos: BlockPos,
    direction: BlockDirection,
) -> Option<i32> {
    let mut cur = pos;
    for _ in 1..search_range {
        if !is_empty_or_water(chunk, cur) {
            break;
        }
        cur = cur.offset(direction.to_offset());
    }
    if !is_empty_or_water(chunk, cur) {
        Some(cur.0.y)
    } else {
        None
    }
}
