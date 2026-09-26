use pumpkin_data::block_properties::{HorizontalFacing, ShelfMushroomProperties};
use pumpkin_data::{Block, BlockDirection, BlockId, BlockState};
use pumpkin_util::math::position::BlockPos;
use pumpkin_util::random::{RandomGenerator, RandomImpl};

use super::super::TreeFeature;
use crate::generation::proto_chunk::GenerationCache;

pub struct ShelfMushroomTreeDecorator {
    pub probability: f32,
}

impl ShelfMushroomTreeDecorator {
    pub fn generate<T: GenerationCache>(
        &self,
        chunk: &mut T,
        random: &mut RandomGenerator,
        log_positions: &[BlockPos],
    ) {
        if random.next_f32() >= self.probability || log_positions.is_empty() {
            return;
        }

        let mut sorted_logs = log_positions.to_vec();
        sorted_logs.sort_by_key(|pos| pos.0.y);

        let is_fallen = sorted_logs.len() > 1 && sorted_logs.first().unwrap().0.y == sorted_logs.last().unwrap().0.y;
        if is_fallen {
            Self::place_on_fallen_log(chunk, random, &sorted_logs);
        } else {
            Self::place_on_standing_tree(chunk, random, &sorted_logs);
        }
    }

    fn place_on_standing_tree<T: GenerationCache>(
        chunk: &mut T,
        random: &mut RandomGenerator,
        logs: &[BlockPos],
    ) {
        let directions = Self::pick_two_perpendicular_directions(random);
        let first_y = logs.first().unwrap().0.y;

        for log_pos in logs {
            let diff = log_pos.0.y - first_y;
            if !(1..=4).contains(&diff) {
                continue;
            }

            for &dir in &directions {
                if random.next_f32() <= 0.25
                    && Self::try_place_mushroom_on_standing_tree(chunk, random, *log_pos, dir)
                {
                    break;
                }
            }
        }
    }

    fn place_on_fallen_log<T: GenerationCache>(
        chunk: &mut T,
        random: &mut RandomGenerator,
        logs: &[BlockPos],
    ) {
        let directions = Self::perpendicular_to_fallen_log(logs);

        for log_pos in logs {
            for &dir in &directions {
                if random.next_f32() <= 0.25 {
                    Self::try_place_mushroom_on_fallen_tree(chunk, random, *log_pos, dir);
                }
            }
        }
    }

    fn try_place_mushroom_on_standing_tree<T: GenerationCache>(
        chunk: &mut T,
        random: &mut RandomGenerator,
        log_pos: BlockPos,
        dir: BlockDirection,
    ) -> bool {
        let mushroom_pos = log_pos.offset(dir.to_offset());
        if !Self::is_block_replaceable_with_shelf_mushroom(chunk, &mushroom_pos) {
            return false;
        }
        if Self::has_shelf_mushroom_at(chunk, &mushroom_pos.down()) {
            return false;
        }
        Self::place_mushroom(chunk, random, mushroom_pos, dir);
        true
    }

    fn try_place_mushroom_on_fallen_tree<T: GenerationCache>(
        chunk: &mut T,
        random: &mut RandomGenerator,
        log_pos: BlockPos,
        dir: BlockDirection,
    ) {
        let mushroom_pos = log_pos.offset(dir.to_offset());
        if !Self::is_block_replaceable_with_shelf_mushroom(chunk, &mushroom_pos) {
            return;
        }
        if Self::has_horizontally_adjacent_shelf_mushroom(chunk, &mushroom_pos) {
            return;
        }
        Self::place_mushroom(chunk, random, mushroom_pos, dir);
    }

    fn pick_two_perpendicular_directions(random: &mut RandomGenerator) -> [BlockDirection; 2] {
        let dir1 = match BlockDirection::random_horizontal(random) {
            HorizontalFacing::North => BlockDirection::North,
            HorizontalFacing::East => BlockDirection::East,
            HorizontalFacing::South => BlockDirection::South,
            HorizontalFacing::West => BlockDirection::West,
        };
        let dir2 = dir1.rotate_clockwise();
        [dir1, dir2]
    }

    fn perpendicular_to_fallen_log(logs: &[BlockPos]) -> [BlockDirection; 2] {
        let first = logs.first().unwrap();
        let last = logs.last().unwrap();
        if first.0.x != last.0.x {
            [BlockDirection::North, BlockDirection::South]
        } else {
            [BlockDirection::East, BlockDirection::West]
        }
    }

    fn has_horizontally_adjacent_shelf_mushroom<T: GenerationCache>(
        chunk: &T,
        pos: &BlockPos,
    ) -> bool {
        for dir in [
            BlockDirection::North,
            BlockDirection::South,
            BlockDirection::East,
            BlockDirection::West,
        ] {
            if Self::has_shelf_mushroom_at(chunk, &pos.offset(dir.to_offset())) {
                return true;
            }
        }
        false
    }

    fn has_shelf_mushroom_at<T: GenerationCache>(chunk: &T, pos: &BlockPos) -> bool {
        let block = GenerationCache::get_block_state(chunk, &pos.0);
        block.to_block_id() == BlockId::SHELF_MUSHROOM
    }

    fn is_block_replaceable_with_shelf_mushroom<T: GenerationCache>(
        chunk: &T,
        pos: &BlockPos,
    ) -> bool {
        let state = GenerationCache::get_block_state(chunk, &pos.0);
        if !TreeFeature::can_replace(state.to_state(), state.to_block_id()) {
            return false;
        }
        if state.to_block_id() == BlockId::WATER {
            return false;
        }
        for dir in [
            BlockDirection::East,
            BlockDirection::West,
            BlockDirection::North,
            BlockDirection::South,
        ] {
            let adj_pos = pos.offset(dir.to_offset());
            let adj_state = GenerationCache::get_block_state(chunk, &adj_pos.0);
            if adj_state.to_block_id() == BlockId::WATER {
                return false;
            }
        }
        true
    }

    fn place_mushroom<T: GenerationCache>(
        chunk: &mut T,
        random: &mut RandomGenerator,
        pos: BlockPos,
        dir: BlockDirection,
    ) {
        let mut props = ShelfMushroomProperties::default(&Block::SHELF_MUSHROOM);
        props.age = random.next_bounded_i32(2) as u8;
        props.facing = match dir {
            BlockDirection::North => HorizontalFacing::North,
            BlockDirection::South => HorizontalFacing::South,
            BlockDirection::East => HorizontalFacing::East,
            BlockDirection::West => HorizontalFacing::West,
            _ => HorizontalFacing::North,
        };
        let state_id = props.to_state_id(&Block::SHELF_MUSHROOM);
        chunk.set_block_state(&pos.0, BlockState::from_id(state_id));
    }
}
