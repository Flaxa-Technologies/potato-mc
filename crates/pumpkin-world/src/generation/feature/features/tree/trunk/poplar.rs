use pumpkin_data::block_properties::Axis;
use pumpkin_data::{Block, BlockDirection, BlockState};
use pumpkin_util::math::int_provider::IntProvider;
use pumpkin_util::math::position::BlockPos;
use pumpkin_util::random::{RandomGenerator, RandomImpl};

use super::TrunkPlacer;
use crate::generation::block_state_provider::BlockStateProvider;
use crate::generation::feature::features::tree::TreeNode;
use crate::generation::proto_chunk::GenerationCache;
use crate::world::WorldPortalExt;

pub struct PoplarTrunkPlacer {
    pub trunk_height_above_branches: IntProvider,
    pub branch_amount: IntProvider,
}

impl PoplarTrunkPlacer {
    #[expect(clippy::too_many_arguments)]
    pub fn generate<T: GenerationCache>(
        &self,
        block_registry: &dyn WorldPortalExt,
        _placer: &TrunkPlacer,
        height: u32,
        start_pos: BlockPos,
        chunk: &mut T,
        random: &mut RandomGenerator,
        below_trunk_provider: &BlockStateProvider,
        trunk_state: &BlockState,
    ) -> (Vec<TreeNode>, Vec<BlockPos>) {
        TrunkPlacer::set_dirt(
            block_registry,
            chunk,
            random,
            &start_pos.down(),
            below_trunk_provider,
        );

        let i = height as i32 - self.trunk_height_above_branches.get(random);
        let mut logs = Vec::new();

        let mut branch_directions = [
            BlockDirection::North,
            BlockDirection::South,
            BlockDirection::East,
            BlockDirection::West,
        ];
        // Fisher-Yates shuffle
        for idx in (1..branch_directions.len()).rev() {
            let j = random.next_bounded_i32((idx + 1) as i32) as usize;
            branch_directions.swap(idx, j);
        }

        for j in 0..height as i32 {
            let pos = start_pos.up_height(j);
            if TrunkPlacer::place(chunk, &pos, trunk_state) {
                logs.push(pos);
            }

            if j == i - 1 {
                let branch_count = self.branch_amount.get(random).clamp(0, 4) as usize;
                for &dir in branch_directions.iter().take(branch_count) {
                    let branch_pos = pos.offset(dir.to_offset());
                    let axis = match dir {
                        BlockDirection::North | BlockDirection::South => Axis::Z,
                        BlockDirection::East | BlockDirection::West => Axis::X,
                        _ => Axis::Y,
                    };
                    let sideways_state = Self::get_sideways_state(trunk_state, axis);
                    if TrunkPlacer::place(chunk, &branch_pos, sideways_state) {
                        logs.push(branch_pos);
                    }
                }
            }
        }

        (
            vec![TreeNode {
                center: start_pos.up_height(i),
                foliage_radius: 0,
                giant_trunk: false,
            }],
            logs,
        )
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
