use piston::PistonBlock;
use pumpkin_data::Block;
use pumpkin_data::BlockDirection;
use pumpkin_data::block_state::PistonBehavior;
use pumpkin_util::math::position::BlockPos;
use pumpkin_world::world::BlockAccessor;

use crate::world::World;

#[expect(clippy::module_inception)]
pub mod piston;
pub mod piston_extension;
pub mod piston_head;

const MAX_MOVABLE_BLOCKS: usize = 12;

pub struct PistonHandler<'a, W: BlockAccessor = World> {
    world: &'a W,
    pos_from: BlockPos,
    extending: bool,
    pos_to: BlockPos,
    motion_direction: BlockDirection,
    pub moved_blocks: Vec<BlockPos>,
    pub broken_blocks: Vec<BlockPos>,
    piston_direction: BlockDirection,
}

impl<'a, W: BlockAccessor> PistonHandler<'a, W> {
    pub fn new(world: &'a W, pos: BlockPos, dir: BlockDirection, extending: bool) -> Self {
        let motion_direction;
        let pos_to = if extending {
            motion_direction = dir;
            pos.offset(dir.to_offset())
        } else {
            motion_direction = dir.opposite();
            pos.offset_dir(dir.to_offset(), 2)
        };
        PistonHandler {
            world,
            pos_from: pos,
            piston_direction: dir,
            extending,
            motion_direction,
            pos_to,
            moved_blocks: Vec::new(),
            broken_blocks: Vec::new(),
        }
    }

    pub fn calculate_push(&mut self) -> bool {
        self.moved_blocks.clear();
        self.broken_blocks.clear();
        let (block, block_state) = self.world.get_block_and_state(&self.pos_to);

        if !PistonBlock::is_movable(
            block,
            block_state,
            self.motion_direction,
            false,
            self.piston_direction,
        ) {
            if self.extending && block_state.piston_behavior == PistonBehavior::Destroy {
                self.broken_blocks.push(self.pos_to);
                return true;
            }
            return false;
        }
        if !self.try_move(self.pos_to, self.motion_direction) {
            return false;
        }
        for i in 0..self.moved_blocks.len() {
            let block_pos = self.moved_blocks[i];
            let block = self.world.get_block(&block_pos);
            if Self::is_block_sticky(block) && !self.try_move_adjacent_block(block, &block_pos) {
                return false;
            }
        }
        true
    }

    pub fn is_block_sticky(block: &Block) -> bool {
        block == &Block::SLIME_BLOCK || block == &Block::HONEY_BLOCK
    }

    pub fn is_adjacent_block_stuck(state: &Block, adjacent_state: &Block) -> bool {
        if state == &Block::HONEY_BLOCK && adjacent_state == &Block::SLIME_BLOCK {
            return false;
        }
        if state == &Block::SLIME_BLOCK && adjacent_state == &Block::HONEY_BLOCK {
            return false;
        }
        Self::is_block_sticky(state) || Self::is_block_sticky(adjacent_state)
    }

    fn is_piston_head_or_base(&self, pos: BlockPos) -> bool {
        pos == self.pos_from
            || (!self.extending && pos == self.pos_from.offset(self.piston_direction.to_offset()))
    }

    fn try_move(&mut self, pos: BlockPos, dir: BlockDirection) -> bool {
        let (mut block, block_state) = self.world.get_block_and_state(&pos);
        if block_state.is_air() {
            return true;
        }
        if !PistonBlock::is_movable(block, block_state, self.motion_direction, false, dir) {
            return true;
        }
        if self.is_piston_head_or_base(pos) {
            return true;
        }
        if self.moved_blocks.contains(&pos) {
            return true;
        }
        let mut i = 1;
        if i + self.moved_blocks.len() > MAX_MOVABLE_BLOCKS {
            return false;
        }
        while Self::is_block_sticky(block) {
            let block_pos = pos.offset_dir(self.motion_direction.opposite().to_offset(), i as i32);
            let block2 = block;
            let (next_block, next_state) = self.world.get_block_and_state(&block_pos);
            if next_state.is_air()
                || !Self::is_adjacent_block_stuck(block2, next_block)
                || !PistonBlock::is_movable(
                    next_block,
                    next_state,
                    self.motion_direction,
                    false,
                    self.motion_direction.opposite(),
                )
                || self.is_piston_head_or_base(block_pos)
            {
                break;
            }
            block = next_block;
            i += 1;
            if i + self.moved_blocks.len() > MAX_MOVABLE_BLOCKS {
                return false;
            }
        }
        let mut j = 0;
        for k in (0..i).rev() {
            self.moved_blocks
                .push(pos.offset_dir(self.motion_direction.opposite().to_offset(), k as i32));
            j += 1;
        }
        let mut k = 1;
        loop {
            let block_pos2 = pos.offset_dir(self.motion_direction.to_offset(), k);
            if let Some(l) = self.moved_blocks.iter().position(|&p| p == block_pos2) {
                self.set_moved_blocks(j, l);
                for m in 0..=(l + j) {
                    let block_pos3 = self.moved_blocks[m];
                    let block = self.world.get_block(&block_pos3);
                    if Self::is_block_sticky(block)
                        && !self.try_move_adjacent_block(block, &block_pos3)
                    {
                        return false;
                    }
                }
                return true;
            }
            let (block, block_state) = self.world.get_block_and_state(&block_pos2);
            if block_state.is_air()
                || (!self.extending
                    && block_pos2 == self.pos_from.offset(self.piston_direction.to_offset()))
            {
                return true;
            }
            if !PistonBlock::is_movable(
                block,
                block_state,
                self.motion_direction,
                true,
                self.motion_direction,
            ) || self.is_piston_head_or_base(block_pos2)
            {
                return false;
            }
            if block_state.piston_behavior == PistonBehavior::Destroy {
                self.broken_blocks.push(block_pos2);
                return true;
            }
            if self.moved_blocks.len() >= MAX_MOVABLE_BLOCKS {
                return false;
            }
            self.moved_blocks.push(block_pos2);
            j += 1;
            k += 1;
        }
    }

    fn set_moved_blocks(&mut self, from: usize, to: usize) {
        let mut list = Vec::new();
        let mut list2 = Vec::new();
        let mut list3 = Vec::new();
        list.extend_from_slice(&self.moved_blocks[0..to]);
        list2.extend_from_slice(&self.moved_blocks[self.moved_blocks.len() - from..]);
        list3.extend_from_slice(&self.moved_blocks[to..self.moved_blocks.len() - from]);
        self.moved_blocks.clear();
        self.moved_blocks.extend(list);
        self.moved_blocks.extend(list2);
        self.moved_blocks.extend(list3);
    }

    fn try_move_adjacent_block(&mut self, block: &Block, pos: &BlockPos) -> bool {
        for direction in BlockDirection::all() {
            if direction.to_axis() == self.motion_direction.to_axis() {
                continue;
            }
            let block_pos = pos.offset(direction.to_offset());
            let block_state2 = self.world.get_block(&block_pos);
            if Self::is_adjacent_block_stuck(block_state2, block)
                && !self.try_move(block_pos, direction)
            {
                return false;
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pumpkin_data::block_properties::StickyPistonLikeProperties;
    use pumpkin_data::{Block, BlockDirection, BlockState, BlockStateId};
    use pumpkin_util::math::position::BlockPos;
    use pumpkin_world::world::BlockAccessor;
    use rustc_hash::FxHashMap;

    struct MockBlockWorld {
        blocks: FxHashMap<BlockPos, (&'static Block, &'static BlockState)>,
    }

    impl MockBlockWorld {
        fn new() -> Self {
            Self {
                blocks: FxHashMap::default(),
            }
        }

        fn set_block(&mut self, pos: BlockPos, block: &'static Block) {
            self.blocks.insert(pos, (block, block.default_state));
        }
    }

    impl BlockAccessor for MockBlockWorld {
        fn get_block(&self, position: &BlockPos) -> &'static Block {
            self.blocks
                .get(position)
                .map(|(b, _)| *b)
                .unwrap_or(&Block::AIR)
        }

        fn get_block_state(&self, position: &BlockPos) -> &'static BlockState {
            self.blocks
                .get(position)
                .map(|(_, s)| *s)
                .unwrap_or(Block::AIR.default_state)
        }

        fn get_block_state_id(&self, position: &BlockPos) -> BlockStateId {
            self.get_block_state(position).id
        }

        fn get_block_and_state(
            &self,
            position: &BlockPos,
        ) -> (&'static Block, &'static BlockState) {
            self.blocks
                .get(position)
                .copied()
                .unwrap_or((&Block::AIR, Block::AIR.default_state))
        }
    }

    #[test]
    fn test_piston_is_movable_rules() {
        let east = BlockDirection::East;

        // Movable normal blocks
        assert!(PistonBlock::is_movable(
            &Block::STONE,
            Block::STONE.default_state,
            east,
            false,
            east
        ));
        assert!(PistonBlock::is_movable(
            &Block::DIRT,
            Block::DIRT.default_state,
            east,
            false,
            east
        ));
        assert!(PistonBlock::is_movable(
            &Block::GLASS,
            Block::GLASS.default_state,
            east,
            false,
            east
        ));
        assert!(PistonBlock::is_movable(
            &Block::AIR,
            Block::AIR.default_state,
            east,
            false,
            east
        ));

        // Immovable hardcoded blocks
        assert!(!PistonBlock::is_movable(
            &Block::OBSIDIAN,
            Block::OBSIDIAN.default_state,
            east,
            false,
            east
        ));
        assert!(!PistonBlock::is_movable(
            &Block::CRYING_OBSIDIAN,
            Block::CRYING_OBSIDIAN.default_state,
            east,
            false,
            east
        ));
        assert!(!PistonBlock::is_movable(
            &Block::RESPAWN_ANCHOR,
            Block::RESPAWN_ANCHOR.default_state,
            east,
            false,
            east
        ));
        assert!(!PistonBlock::is_movable(
            &Block::REINFORCED_DEEPSLATE,
            Block::REINFORCED_DEEPSLATE.default_state,
            east,
            false,
            east
        ));

        // Hardness == -1.0 (unbreakable blocks like Bedrock)
        assert!(!PistonBlock::is_movable(
            &Block::BEDROCK,
            Block::BEDROCK.default_state,
            east,
            false,
            east
        ));

        // Extended piston is immovable; retracted piston is movable
        let mut props = StickyPistonLikeProperties::default(&Block::PISTON);
        props.extended = true;
        let ext_state = BlockState::from_id(props.to_state_id(&Block::PISTON));
        assert!(!PistonBlock::is_movable(
            &Block::PISTON,
            ext_state,
            east,
            false,
            east
        ));

        props.extended = false;
        let retracted_state = BlockState::from_id(props.to_state_id(&Block::PISTON));
        assert!(PistonBlock::is_movable(
            &Block::PISTON,
            retracted_state,
            east,
            false,
            east
        ));

        // Fragile blocks (Destroy reaction)
        assert!(PistonBlock::is_movable(
            &Block::REDSTONE_TORCH,
            Block::REDSTONE_TORCH.default_state,
            east,
            true,
            east
        ));
        assert!(!PistonBlock::is_movable(
            &Block::REDSTONE_TORCH,
            Block::REDSTONE_TORCH.default_state,
            east,
            false,
            east
        ));
    }

    #[test]
    fn test_piston_push_limit_12_blocks_success() {
        let mut world = MockBlockWorld::new();
        let piston_pos = BlockPos::new(0, 64, 0);
        let push_dir = BlockDirection::East;

        // Setup 12 stone blocks in a row from (1, 64, 0) to (12, 64, 0)
        for x in 1..=12 {
            world.set_block(BlockPos::new(x, 64, 0), &Block::STONE);
        }
        world.set_block(BlockPos::new(13, 64, 0), &Block::AIR);

        let mut handler = PistonHandler::new(&world, piston_pos, push_dir, true);
        assert!(
            handler.calculate_push(),
            "Piston must successfully push a line of 12 blocks"
        );
        assert_eq!(
            handler.moved_blocks.len(),
            12,
            "Moved blocks count must be exactly 12"
        );
        assert!(handler.broken_blocks.is_empty());
    }

    #[test]
    fn test_piston_push_limit_13_blocks_rejected() {
        let mut world = MockBlockWorld::new();
        let piston_pos = BlockPos::new(0, 64, 0);
        let push_dir = BlockDirection::East;

        // Setup 13 stone blocks in a row from (1, 64, 0) to (13, 64, 0)
        for x in 1..=13 {
            world.set_block(BlockPos::new(x, 64, 0), &Block::STONE);
        }
        world.set_block(BlockPos::new(14, 64, 0), &Block::AIR);

        let mut handler = PistonHandler::new(&world, piston_pos, push_dir, true);
        assert!(
            !handler.calculate_push(),
            "Piston must refuse to push 13 or more blocks"
        );
    }

    #[test]
    fn test_piston_push_line_halted_by_immovable_obsidian() {
        let mut world = MockBlockWorld::new();
        let piston_pos = BlockPos::new(0, 64, 0);
        let push_dir = BlockDirection::East;

        // Setup 3 stone blocks followed by obsidian
        world.set_block(BlockPos::new(1, 64, 0), &Block::STONE);
        world.set_block(BlockPos::new(2, 64, 0), &Block::STONE);
        world.set_block(BlockPos::new(3, 64, 0), &Block::OBSIDIAN);
        world.set_block(BlockPos::new(4, 64, 0), &Block::AIR);

        let mut handler = PistonHandler::new(&world, piston_pos, push_dir, true);
        assert!(
            !handler.calculate_push(),
            "Piston must refuse to push when line contains Obsidian"
        );
    }

    #[test]
    fn test_piston_push_line_halted_by_bedrock() {
        let mut world = MockBlockWorld::new();
        let piston_pos = BlockPos::new(0, 64, 0);
        let push_dir = BlockDirection::East;

        world.set_block(BlockPos::new(1, 64, 0), &Block::BEDROCK);

        let mut handler = PistonHandler::new(&world, piston_pos, push_dir, true);
        assert!(
            !handler.calculate_push(),
            "Piston must refuse to push Bedrock"
        );
    }

    #[test]
    fn test_piston_fragile_torch_destroyed_directly_in_front() {
        let mut world = MockBlockWorld::new();
        let piston_pos = BlockPos::new(0, 64, 0);
        let push_dir = BlockDirection::East;

        // Fragile torch directly in front of piston
        world.set_block(BlockPos::new(1, 64, 0), &Block::REDSTONE_TORCH);

        let mut handler = PistonHandler::new(&world, piston_pos, push_dir, true);
        assert!(
            handler.calculate_push(),
            "Piston must push and break fragile block directly in front"
        );
        assert!(handler.moved_blocks.is_empty());
        assert_eq!(
            handler.broken_blocks,
            vec![BlockPos::new(1, 64, 0)],
            "Torch must be in broken_blocks"
        );
    }

    #[test]
    fn test_piston_12_blocks_push_and_break_13th_fragile_torch() {
        let mut world = MockBlockWorld::new();
        let piston_pos = BlockPos::new(0, 64, 0);
        let push_dir = BlockDirection::East;

        // 12 stone blocks, followed by redstone torch at position 13
        for x in 1..=12 {
            world.set_block(BlockPos::new(x, 64, 0), &Block::STONE);
        }
        world.set_block(BlockPos::new(13, 64, 0), &Block::REDSTONE_TORCH);

        let mut handler = PistonHandler::new(&world, piston_pos, push_dir, true);
        assert!(
            handler.calculate_push(),
            "Piston pushing 12 blocks into fragile block must succeed by breaking fragile block"
        );
        assert_eq!(
            handler.moved_blocks.len(),
            12,
            "All 12 stone blocks must be moved"
        );
        assert_eq!(
            handler.broken_blocks,
            vec![BlockPos::new(13, 64, 0)],
            "Fragile torch must be broken without consuming push limit"
        );
    }

    #[test]
    fn test_sticky_piston_retraction_pulls_block_north() {
        let mut world = MockBlockWorld::new();
        let piston_pos = BlockPos::new(0, 64, 0);
        let dir = BlockDirection::North;
        // North offset is (0, 0, -1). 2 blocks ahead is (0, 64, -2)
        let block_pos = piston_pos.offset_dir(dir.to_offset(), 2);
        world.set_block(block_pos, &Block::STONE);

        let mut handler = PistonHandler::new(&world, piston_pos, dir, false);
        assert!(
            handler.calculate_push(),
            "Sticky piston retracting North must successfully pull block"
        );
        assert_eq!(
            handler.moved_blocks,
            vec![block_pos],
            "North sticky piston must pull stone block from distance 2"
        );
        assert!(handler.broken_blocks.is_empty());
    }

    #[test]
    fn test_sticky_piston_retraction_all_directions() {
        for dir in BlockDirection::all() {
            let mut world = MockBlockWorld::new();
            let piston_pos = BlockPos::new(10, 64, 10);
            let pull_pos = piston_pos.offset_dir(dir.to_offset(), 2);
            world.set_block(pull_pos, &Block::STONE);

            let mut handler = PistonHandler::new(&world, piston_pos, dir, false);
            assert!(
                handler.calculate_push(),
                "Sticky piston retracting in direction {dir:?} must calculate push"
            );
            assert_eq!(
                handler.moved_blocks,
                vec![pull_pos],
                "Retracted block in direction {dir:?} must be moved"
            );

            // Verify pull direction check: motion direction during retraction is dir.opposite()
            assert!(
                PistonBlock::is_movable(
                    &Block::STONE,
                    Block::STONE.default_state,
                    dir.opposite(),
                    false,
                    dir
                ),
                "Stone must be movable in pull direction {dir:?}.opposite()"
            );
        }
    }

    #[test]
    fn test_sticky_piston_retraction_leaves_immovable() {
        let mut world = MockBlockWorld::new();
        let piston_pos = BlockPos::new(0, 64, 0);
        let dir = BlockDirection::North;
        let obsidian_pos = piston_pos.offset_dir(dir.to_offset(), 2);
        world.set_block(obsidian_pos, &Block::OBSIDIAN);

        let mut handler = PistonHandler::new(&world, piston_pos, dir, false);
        assert!(
            !handler.calculate_push(),
            "Sticky piston must not pull immovable obsidian block"
        );
        assert!(handler.moved_blocks.is_empty());
    }

    #[test]
    fn test_sticky_piston_retraction_leaves_fragile() {
        let mut world = MockBlockWorld::new();
        let piston_pos = BlockPos::new(0, 64, 0);
        let dir = BlockDirection::North;
        let torch_pos = piston_pos.offset_dir(dir.to_offset(), 2);
        world.set_block(torch_pos, &Block::REDSTONE_TORCH);

        let mut handler = PistonHandler::new(&world, piston_pos, dir, false);
        assert!(
            !handler.calculate_push(),
            "Sticky piston must not pull fragile block during retraction"
        );
        assert!(handler.moved_blocks.is_empty());
    }

    #[test]
    fn test_slime_and_honey_stickiness_rules() {
        assert!(PistonHandler::<World>::is_block_sticky(&Block::SLIME_BLOCK));
        assert!(PistonHandler::<World>::is_block_sticky(&Block::HONEY_BLOCK));
        assert!(!PistonHandler::<World>::is_block_sticky(&Block::STONE));

        // Vanilla rule: Slime and Honey do NOT stick to each other
        assert!(!PistonHandler::<World>::is_adjacent_block_stuck(
            &Block::SLIME_BLOCK,
            &Block::HONEY_BLOCK
        ));
        assert!(!PistonHandler::<World>::is_adjacent_block_stuck(
            &Block::HONEY_BLOCK,
            &Block::SLIME_BLOCK
        ));

        // Slime sticks to stone, Honey sticks to stone
        assert!(PistonHandler::<World>::is_adjacent_block_stuck(
            &Block::SLIME_BLOCK,
            &Block::STONE
        ));
        assert!(PistonHandler::<World>::is_adjacent_block_stuck(
            &Block::HONEY_BLOCK,
            &Block::STONE
        ));
    }

    #[test]
    fn test_north_direction_index_parity() {
        // Direct verification that North index == 2 in Pumpkin
        assert_eq!(
            BlockDirection::North.to_index(),
            2,
            "North direction index must be 2, confirming why `if data == 2` broke all North sticky pistons"
        );
    }
}
