use pumpkin_data::block_properties::Axis;
use pumpkin_data::{Block, BlockState};
use pumpkin_util::{
    math::{int_provider::IntProvider, position::BlockPos, vector3::Vector3},
    random::{RandomGenerator, RandomImpl},
};

use super::FoliagePlacer;
use crate::generation::feature::features::tree::TreeNode;
use crate::generation::proto_chunk::GenerationCache;

pub struct PoplarFoliagePlacer {
    pub height: IntProvider,
    pub side_hole_chance: f32,
}

impl PoplarFoliagePlacer {
    #[expect(clippy::too_many_arguments)]
    pub fn generate<T: GenerationCache>(
        &self,
        chunk: &mut T,
        random: &mut RandomGenerator,
        node: &TreeNode,
        foliage_height: i32,
        radius: i32,
        offset: i32,
        foliage_provider: &BlockState,
    ) -> Vec<BlockPos> {
        let giant_trunk = node.giant_trunk;
        let center_pos = node.center.up_height(offset);
        let radius = radius + node.foliage_radius - 1;
        let random_boolean = random.next_bounded_i32(2) == 0;
        let total_foliage_height = foliage_height;
        let mut foliage_positions = Vec::new();

        // 1. Layer total_foliage_height - 1 (radius - 2)
        self.place_leaves_row(
            chunk,
            random,
            center_pos,
            radius - 2,
            total_foliage_height - 1,
            giant_trunk,
            total_foliage_height,
            random_boolean,
            foliage_provider,
            &mut foliage_positions,
        );

        // 2. Layer total_foliage_height - 2 (radius - 1)
        self.place_leaves_row(
            chunk,
            random,
            center_pos,
            radius - 1,
            total_foliage_height - 2,
            giant_trunk,
            total_foliage_height,
            random_boolean,
            foliage_provider,
            &mut foliage_positions,
        );

        // 3. Layer total_foliage_height - 3 (radius - 1)
        self.place_leaves_row(
            chunk,
            random,
            center_pos,
            radius - 1,
            total_foliage_height - 3,
            giant_trunk,
            total_foliage_height,
            random_boolean,
            foliage_provider,
            &mut foliage_positions,
        );

        // 4. Middle layers: from total_foliage_height - 4 down to 1 (radius)
        for y in (1..=total_foliage_height - 4).rev() {
            self.place_leaves_row(
                chunk,
                random,
                center_pos,
                radius,
                y,
                giant_trunk,
                total_foliage_height,
                random_boolean,
                foliage_provider,
                &mut foliage_positions,
            );
        }

        // 5. Replace leaves with log at layer total_foliage_height - 4
        Self::replace_leaves_with_log(
            chunk,
            center_pos,
            radius,
            total_foliage_height - 4,
            giant_trunk,
            total_foliage_height,
            random_boolean,
            foliage_provider,
        );

        // 6. Layer 0 (radius - 1)
        self.place_leaves_row(
            chunk,
            random,
            center_pos,
            radius - 1,
            0,
            giant_trunk,
            total_foliage_height,
            random_boolean,
            foliage_provider,
            &mut foliage_positions,
        );

        // 7. Layer -1 ((radius - 2).clamp(1, 2))
        let bottom_radius = (radius - 2).clamp(1, 2);
        self.place_leaves_row(
            chunk,
            random,
            center_pos,
            bottom_radius,
            -1,
            giant_trunk,
            total_foliage_height,
            random_boolean,
            foliage_provider,
            &mut foliage_positions,
        );

        foliage_positions
    }

    #[must_use]
    pub fn get_random_height(&self, random: &mut RandomGenerator) -> i32 {
        self.height.get(random)
    }

    #[expect(clippy::too_many_arguments)]
    fn place_leaves_row<T: GenerationCache>(
        &self,
        chunk: &mut T,
        random: &mut RandomGenerator,
        center_pos: BlockPos,
        radius: i32,
        y: i32,
        giant_trunk: bool,
        total_foliage_height: i32,
        random_boolean: bool,
        foliage_provider: &BlockState,
        foliage_positions: &mut Vec<BlockPos>,
    ) {
        let i = i32::from(giant_trunk);
        for dx in -radius..=(radius + i) {
            for dz in -radius..=(radius + i) {
                if self.should_skip_location(
                    random,
                    dx,
                    y,
                    dz,
                    radius,
                    total_foliage_height,
                    random_boolean,
                ) {
                    continue;
                }
                let pos = BlockPos(center_pos.0.add(&Vector3::new(dx, y, dz)));
                if FoliagePlacer::place_foliage_block(chunk, pos, foliage_provider) {
                    foliage_positions.push(pos);
                }
            }
        }
    }

    fn should_skip_location(
        &self,
        random: &mut RandomGenerator,
        dx: i32,
        y: i32,
        dz: i32,
        radius: i32,
        total_foliage_height: i32,
        random_boolean: bool,
    ) -> bool {
        let is_partial = Self::should_row_be_partial_rhombus_shape(total_foliage_height, y);
        let corner_blocks = Self::get_corner_blocks_to_cut_for_rhombus_shape(
            dx,
            dz,
            radius,
            is_partial,
            random_boolean,
        );
        let abs_x = dx.abs();
        let abs_z = dz.abs();
        let at_edge = abs_x == radius || abs_z == radius;
        if is_partial && at_edge {
            return true;
        }
        let side_hole = if random.next_f32() <= self.side_hole_chance {
            1
        } else {
            0
        };
        !Self::is_within_rhombus_shape(radius, abs_x, abs_z, corner_blocks, side_hole)
    }

    const fn should_row_be_partial_rhombus_shape(total_foliage_height: i32, y: i32) -> bool {
        y == total_foliage_height - 1 || y == total_foliage_height - 2
    }

    const fn get_corner_blocks_to_cut_for_rhombus_shape(
        dx: i32,
        dz: i32,
        radius: i32,
        is_partial: bool,
        random_boolean: bool,
    ) -> i32 {
        let flag = if random_boolean {
            Self::is_left_top_corner_or_right_lower_corner(dx, dz)
        } else {
            Self::is_left_lower_corner_or_right_top_corner(dx, dz)
        };
        if flag {
            radius - 1
        } else if is_partial {
            radius + 1
        } else {
            radius
        }
    }

    const fn is_left_lower_corner_or_right_top_corner(dx: i32, dz: i32) -> bool {
        (dx > 0 && dz < 0) || (dx < 0 && dz > 0)
    }

    const fn is_left_top_corner_or_right_lower_corner(dx: i32, dz: i32) -> bool {
        (dx > 0 && dz > 0) || (dx < 0 && dz < 0)
    }

    const fn is_within_rhombus_shape(
        radius: i32,
        abs_x: i32,
        abs_z: i32,
        corner_blocks: i32,
        side_hole: i32,
    ) -> bool {
        abs_x + abs_z <= radius * 2 - (corner_blocks + side_hole)
    }

    #[expect(clippy::too_many_arguments)]
    fn replace_leaves_with_log<T: GenerationCache>(
        chunk: &mut T,
        center_pos: BlockPos,
        radius: i32,
        y: i32,
        giant_trunk: bool,
        total_foliage_height: i32,
        random_boolean: bool,
        foliage_provider: &BlockState,
    ) {
        let i = i32::from(giant_trunk);
        let is_partial = Self::should_row_be_partial_rhombus_shape(total_foliage_height, y);

        // Find trunk log state from tree trunk below center
        let trunk_pos = center_pos.down();
        let trunk_state_id = GenerationCache::get_block_state(chunk, &trunk_pos.0);
        let trunk_state = BlockState::from_id(trunk_state_id);

        for dx in -radius..=(radius + i) {
            for dz in -radius..=(radius + i) {
                let abs_x = dx.abs();
                let abs_z = dz.abs();
                let corner_blocks = Self::get_corner_blocks_to_cut_for_rhombus_shape(
                    dx,
                    dz,
                    radius,
                    is_partial,
                    random_boolean,
                );
                if Self::is_within_rhombus_shape(radius, abs_x, abs_z, corner_blocks, 2)
                    && ((abs_z == 0 && radius - abs_x >= 4) || (abs_x == 0 && radius - abs_z >= 4))
                {
                    let pos = BlockPos(center_pos.0.add(&Vector3::new(dx, y, dz)));
                    if GenerationCache::get_block_state(chunk, &pos.0) == foliage_provider.id {
                        let axis = if abs_z == 0 { Axis::X } else { Axis::Z };
                        let sideways_state = Self::get_sideways_state(trunk_state, axis);
                        chunk.set_block_state(&pos.0, sideways_state);
                    }
                }
            }
        }
    }

    fn get_sideways_state(trunk_state: &BlockState, axis: Axis) -> &'static BlockState {
        let block = Block::from_state_id(trunk_state.id);
        if let Some(props_source) = block.properties(trunk_state.id) {
            let mut props = props_source.to_props();
            let axis_str = match axis {
                Axis::X => "x",
                Axis::Y => "y",
                Axis::Z => "z",
            };
            if let Some(idx) = props.iter().position(|(k, _)| *k == "axis") {
                props[idx] = ("axis", axis_str);
            } else {
                props.push(("axis", axis_str));
            }
            let new_state_id = block.from_properties(&props).to_state_id(block);
            return BlockState::from_id(new_state_id);
        }
        BlockState::from_id(trunk_state.id)
    }
}
