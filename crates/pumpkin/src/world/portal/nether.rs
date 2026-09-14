use super::poi;
use pumpkin_data::{
    Block, BlockDirection, BlockState,
    block_properties::{HorizontalAxis, NetherPortalLikeProperties},
    tag,
    tag::Taggable,
};
use pumpkin_util::math::{boundingbox::EntityDimensions, position::BlockPos, vector3::Vector3};
use pumpkin_world::{chunk::ChunkHeightmapType, world::BlockFlags};
use std::sync::Arc;

use crate::world::World;

pub(crate) const SEARCH_RADIUS_NETHER: i32 = 16;
pub(crate) const SEARCH_RADIUS_OVERWORLD: i32 = 128;

#[derive(Debug, Clone)]
pub struct SpiralIterator {
    legs: i32,
    leg: i32,
    leg_size: i32,
    leg_index: i32,
    last_x: i32,
    last_z: i32,
}

impl SpiralIterator {
    #[must_use]
    pub const fn new(center_x: i32, center_z: i32, radius: i32) -> Self {
        Self {
            legs: 4 * radius,
            leg: -1,
            leg_size: 0,
            leg_index: 0,
            last_x: center_x,
            last_z: center_z + 1,
        }
    }
}

impl Iterator for SpiralIterator {
    type Item = (i32, i32);

    fn next(&mut self) -> Option<Self::Item> {
        const DIRS: [(i32, i32); 4] = [(1, 0), (0, 1), (-1, 0), (0, -1)];

        let dir_idx = (self.leg + 4).rem_euclid(4) as usize;
        let (dx, dz) = DIRS[dir_idx];
        self.last_x += dx;
        self.last_z += dz;

        if self.leg_index >= self.leg_size {
            if self.leg >= self.legs {
                return None;
            }
            self.leg += 1;
            self.leg_index = 0;
            self.leg_size = self.leg / 2 + 1;
        }

        self.leg_index += 1;
        Some((self.last_x, self.last_z))
    }
}

#[derive(Debug, Clone)]
pub struct PortalSearchResult {
    pub lower_corner: BlockPos,
    pub axis: HorizontalAxis,
    pub width: u32,
    pub height: u32,
}

impl PortalSearchResult {
    #[must_use]
    pub fn get_teleport_position(&self) -> Vector3<f64> {
        let x = f64::from(self.lower_corner.0.x);
        let y = f64::from(self.lower_corner.0.y);
        let z = f64::from(self.lower_corner.0.z);

        match self.axis {
            HorizontalAxis::X => Vector3::new(x + f64::from(self.width) / 2.0, y, z + 0.5),
            HorizontalAxis::Z => Vector3::new(x + 0.5, y, z + f64::from(self.width) / 2.0),
        }
    }

    /// Calculates the yaw adjustment when teleporting between portals with different axes.
    /// Returns the new yaw value for the entity.
    #[must_use]
    pub fn calculate_teleport_yaw(
        &self,
        current_yaw: f32,
        source_axis: Option<HorizontalAxis>,
    ) -> f32 {
        let Some(src_axis) = source_axis else {
            return current_yaw;
        };

        if src_axis == self.axis {
            return current_yaw;
        }

        // Axis changed, rotate yaw by 90 degrees
        // X axis portal faces East/West, Z axis portal faces North/South
        match (src_axis, self.axis) {
            (HorizontalAxis::X, HorizontalAxis::Z) => current_yaw + 90.0,
            (HorizontalAxis::Z, HorizontalAxis::X) => current_yaw - 90.0,
            _ => current_yaw,
        }
    }

    #[must_use]
    pub fn entity_pos_in_portal(
        &self,
        entity_pos: Vector3<f64>,
        dimensions: &EntityDimensions,
    ) -> Vector3<f64> {
        let portal_width = f64::from(self.width) - f64::from(dimensions.width);
        let portal_height = f64::from(self.height) - f64::from(dimensions.height);
        let lower = self.lower_corner.0;

        let axis_progress = if portal_width > 0.0 {
            let axis_coord = match self.axis {
                HorizontalAxis::X => entity_pos.x,
                HorizontalAxis::Z => entity_pos.z,
            };
            let lower_axis = match self.axis {
                HorizontalAxis::X => f64::from(lower.x),
                HorizontalAxis::Z => f64::from(lower.z),
            };
            let offset = axis_coord - (lower_axis + f64::from(dimensions.width) / 2.0);
            (offset / portal_width).clamp(0.0, 1.0)
        } else {
            0.5
        };

        let y_progress = if portal_height > 0.0 {
            let offset = entity_pos.y - f64::from(lower.y);
            (offset / portal_height).clamp(0.0, 1.0)
        } else {
            0.0
        };

        let perp_offset = match self.axis {
            HorizontalAxis::X => entity_pos.z - (f64::from(lower.z) + 0.5),
            HorizontalAxis::Z => entity_pos.x - (f64::from(lower.x) + 0.5),
        };
        // Clamp perpendicular offset to keep exit position within portal bounds
        // (prevents spawning inside solid blocks next to the portal)
        let perp_offset = perp_offset.clamp(-0.5, 0.5);

        Vector3::new(axis_progress, y_progress, perp_offset)
    }

    #[must_use]
    pub fn calculate_exit_position(
        &self,
        relative_pos: Vector3<f64>,
        dimensions: &EntityDimensions,
    ) -> Vector3<f64> {
        let portal_width = f64::from(self.width) - f64::from(dimensions.width);
        let portal_height = f64::from(self.height) - f64::from(dimensions.height);
        let lower = self.lower_corner.0;

        let axis_offset = if portal_width > 0.0 {
            relative_pos
                .x
                .mul_add(portal_width, f64::from(dimensions.width) / 2.0)
        } else {
            f64::from(self.width) / 2.0
        };

        let y_offset = if portal_height > 0.0 {
            relative_pos.y * portal_height
        } else {
            0.0
        };

        match self.axis {
            HorizontalAxis::X => Vector3::new(
                f64::from(lower.x) + axis_offset,
                f64::from(lower.y) + y_offset,
                f64::from(lower.z) + 0.5 + relative_pos.z,
            ),
            HorizontalAxis::Z => Vector3::new(
                f64::from(lower.x) + 0.5 + relative_pos.z,
                f64::from(lower.y) + y_offset,
                f64::from(lower.z) + axis_offset,
            ),
        }
    }

    pub fn find_open_position(
        &self,
        world: &Arc<World>,
        fallback: Vector3<f64>,
        dimensions: &EntityDimensions,
    ) -> Vector3<f64> {
        if dimensions.width > 4.0 || dimensions.height > 4.0 {
            return fallback;
        }

        let half_height = f64::from(dimensions.height) / 2.0;
        let check_pos = Vector3::new(fallback.x, fallback.y + half_height, fallback.z);

        if Self::is_position_clear(world, check_pos, dimensions) {
            return fallback;
        }

        let search_radius = 1.0;
        let step = 0.5;

        let mut best_pos = fallback;
        let mut best_dist = f64::MAX;

        let mut dx = -search_radius;
        while dx <= search_radius {
            let mut dz = -search_radius;
            while dz <= search_radius {
                let test_pos = Vector3::new(check_pos.x + dx, check_pos.y, check_pos.z + dz);
                if Self::is_position_clear(world, test_pos, dimensions) {
                    let dist = dx * dx + dz * dz;
                    if dist < best_dist {
                        best_dist = dist;
                        best_pos = Vector3::new(test_pos.x, fallback.y, test_pos.z);
                    }
                }
                dz += step;
            }
            dx += step;
        }

        best_pos
    }

    fn is_position_clear(
        world: &Arc<World>,
        center: Vector3<f64>,
        dimensions: &EntityDimensions,
    ) -> bool {
        let half_width = f64::from(dimensions.width) / 2.0;
        let height = f64::from(dimensions.height);

        // Calculate the bounding box in block coordinates
        let min_x = (center.x - half_width).floor() as i32;
        let max_x = (center.x + half_width).floor() as i32;
        let min_y = (center.y - height / 2.0).floor() as i32;
        let max_y = (center.y + height / 2.0).floor() as i32;
        let min_z = (center.z - half_width).floor() as i32;
        let max_z = (center.z + half_width).floor() as i32;

        // Check ALL blocks that overlap with the entity bounding box
        for x in min_x..=max_x {
            for y in min_y..=max_y {
                for z in min_z..=max_z {
                    let block_pos = BlockPos(Vector3::new(x, y, z));
                    let state = world.get_block_state(&block_pos);
                    if state.is_solid_block() {
                        return false;
                    }
                }
            }
        }
        true
    }
}

pub struct NetherPortal {
    axis: HorizontalAxis,
    found_portal_blocks: u32,
    negative_direction: BlockDirection,
    lower_conor: BlockPos,
    width: u32,
    height: u32,
}

impl NetherPortal {
    const MIN_WIDTH: u32 = 2;
    const MAX_WIDTH: u32 = 21;
    const MAX_HEIGHT: u32 = 21;
    const MIN_HEIGHT: u32 = 3;
    const FRAME_BLOCK: Block = Block::OBSIDIAN;

    #[must_use]
    pub const fn is_valid(&self) -> bool {
        self.width >= Self::MIN_WIDTH
            && self.width <= Self::MAX_WIDTH
            && self.height >= Self::MIN_HEIGHT
            && self.height <= Self::MAX_HEIGHT
    }

    #[must_use]
    pub const fn was_already_valid(&self) -> bool {
        self.is_valid() && self.found_portal_blocks == self.width * self.height
    }

    #[must_use]
    pub const fn lower_corner(&self) -> BlockPos {
        self.lower_conor
    }

    #[must_use]
    pub const fn axis(&self) -> HorizontalAxis {
        self.axis
    }

    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }

    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }

    pub fn create(&self, world: &Arc<World>) {
        let mut props = NetherPortalLikeProperties::default(&Block::NETHER_PORTAL);
        props.axis = self.axis;
        let state = props.to_state_id(&Block::NETHER_PORTAL);
        let blocks = BlockPos::iterate(
            self.lower_conor,
            self.lower_conor
                .offset_dir(BlockDirection::Up.to_offset(), self.height as i32 - 1)
                .offset_dir(self.negative_direction.to_offset(), self.width as i32 - 1),
        );

        for pos in blocks {
            world.set_block_state(
                &pos,
                state,
                BlockFlags::NOTIFY_LISTENERS | BlockFlags::FORCE_STATE,
            );
            world
                .portal_poi
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .add_portal(pos);
        }
    }

    pub fn get_new_portal(
        world: &World,
        pos: &BlockPos,
        first_axis: HorizontalAxis,
    ) -> Option<Self> {
        if let Some(portal) = Self::get_on_axis(world, pos, first_axis)
            && portal.is_valid()
            && portal.found_portal_blocks == 0
        {
            return Some(portal);
        }
        let next_axis = if first_axis == HorizontalAxis::X {
            HorizontalAxis::Z
        } else {
            HorizontalAxis::X
        };
        if let Some(portal) = Self::get_on_axis(world, pos, next_axis)
            && portal.is_valid()
            && portal.found_portal_blocks == 0
        {
            return Some(portal);
        }
        None
    }

    pub fn get_on_axis(world: &World, pos: &BlockPos, axis: HorizontalAxis) -> Option<Self> {
        let (block, state) = world.get_block_and_state(pos);
        if block == &Block::NETHER_PORTAL {
            let props = NetherPortalLikeProperties::from_state_id(state.id);
            if props.axis != axis {
                return None;
            }
        }
        let direction = if axis == HorizontalAxis::X {
            BlockDirection::East
        } else {
            BlockDirection::South
        };
        let cornor = Self::get_lower_cornor(world, direction, pos, axis)?;
        let width = Self::get_width(world, &cornor, direction, axis);
        if !(Self::MIN_WIDTH..=Self::MAX_WIDTH).contains(&width) {
            return None;
        }
        let mut found_portal_blocks = 0;
        let height = Self::get_height(
            world,
            &cornor,
            direction,
            width,
            &mut found_portal_blocks,
            axis,
        )?;
        Some(Self {
            axis,
            found_portal_blocks,
            negative_direction: direction,
            lower_conor: cornor,
            width,
            height,
        })
    }

    fn get_lower_cornor(
        world: &World,
        direction: BlockDirection,
        pos: &BlockPos,
        axis: HorizontalAxis,
    ) -> Option<BlockPos> {
        let limit_y = pos.0.y - Self::MAX_HEIGHT as i32;
        let mut pos = *pos;
        while pos.0.y > limit_y {
            let (block, state) = world.get_block_and_state(&pos.down());
            if !Self::valid_state_inside_portal(block, state, axis) {
                break;
            }
            pos = pos.down();
        }
        let neg_dir = direction.opposite();
        let width = (Self::get_width(world, &pos, neg_dir, axis) as i32) - 1;
        if width < 0 {
            return None;
        }
        Some(pos.offset_dir(neg_dir.to_offset(), width))
    }

    fn get_width(
        world: &World,
        original_lower_corner: &BlockPos,
        negative_dir: BlockDirection,
        axis: HorizontalAxis,
    ) -> u32 {
        let mut lower_corner;
        for i in 0..=Self::MAX_WIDTH {
            lower_corner = original_lower_corner.offset_dir(negative_dir.to_offset(), i as i32);
            let (block, block_state) = world.get_block_and_state(&lower_corner);
            if !Self::valid_state_inside_portal(block, block_state, axis) {
                if &Self::FRAME_BLOCK != block {
                    break;
                }
                return i;
            }
            let block = world.get_block(&lower_corner.down());
            if &Self::FRAME_BLOCK != block {
                break;
            }
        }
        0
    }

    fn get_height(
        world: &World,
        lower_corner: &BlockPos,
        negative_dir: BlockDirection,
        width: u32,
        found_portal_blocks: &mut u32,
        axis: HorizontalAxis,
    ) -> Option<u32> {
        let height = Self::get_potential_height(
            world,
            lower_corner,
            negative_dir,
            width,
            found_portal_blocks,
            axis,
        );
        if !(Self::MIN_HEIGHT..=Self::MAX_HEIGHT).contains(&height)
            || !Self::is_horizontal_frame_valid(world, lower_corner, negative_dir, width, height)
        {
            return None;
        }
        Some(height)
    }

    fn get_potential_height(
        world: &World,
        lower_corner: &BlockPos,
        negative_dir: BlockDirection,
        width: u32,
        found_portal_blocks: &mut u32,
        axis: HorizontalAxis,
    ) -> u32 {
        for i in 0..Self::MAX_HEIGHT as i32 {
            let mut pos = lower_corner
                .offset_dir(BlockDirection::Up.to_offset(), i)
                .offset_dir(negative_dir.to_offset(), -1);
            if world.get_block(&pos) != &Self::FRAME_BLOCK {
                return i as u32;
            }

            pos = lower_corner
                .offset_dir(BlockDirection::Up.to_offset(), i)
                .offset_dir(negative_dir.to_offset(), width as i32);
            if world.get_block(&pos) != &Self::FRAME_BLOCK {
                return i as u32;
            }

            for j in 0..width {
                pos = lower_corner
                    .offset_dir(BlockDirection::Up.to_offset(), i)
                    .offset_dir(negative_dir.to_offset(), j as i32);
                let (block, block_state) = world.get_block_and_state(&pos);
                if !Self::valid_state_inside_portal(block, block_state, axis) {
                    return i as u32;
                }
                if block == &Block::NETHER_PORTAL {
                    *found_portal_blocks += 1;
                }
            }
        }
        21
    }

    fn is_horizontal_frame_valid(
        world: &World,
        lower_corner: &BlockPos,
        dir: BlockDirection,
        width: u32,
        height: u32,
    ) -> bool {
        let mut pos;
        for i in 0..width {
            pos = lower_corner
                .offset_dir(BlockDirection::Up.to_offset(), height as i32)
                .offset_dir(dir.to_offset(), i as i32);
            if &Self::FRAME_BLOCK != world.get_block(&pos) {
                return false;
            }
        }
        true
    }

    fn valid_state_inside_portal(block: &Block, state: &BlockState, axis: HorizontalAxis) -> bool {
        if block == &Block::NETHER_PORTAL {
            let props = NetherPortalLikeProperties::from_state_id(state.id);
            props.axis == axis
        } else {
            state.is_air() || block.has_tag(&tag::Block::MINECRAFT_FIRE)
        }
    }

    pub fn search_for_portal(
        world: &Arc<World>,
        target_pos: BlockPos,
    ) -> Option<PortalSearchResult> {
        tracing::debug!(
            "Searching for portal in {:?} around {:?}",
            world.dimension.minecraft_name,
            target_pos
        );
        let min_y = world.min_y;
        let max_y = min_y + world.dimension.height - 1;
        let worldborder = world
            .worldborder
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        let search_radius = if world.dimension.has_ceiling {
            SEARCH_RADIUS_NETHER
        } else {
            SEARCH_RADIUS_OVERWORLD
        };

        let search_max_y = if world.dimension.has_ceiling {
            (min_y + world.dimension.logical_height - 1).min(max_y)
        } else {
            max_y
        };

        let portal_positions = {
            let mut poi_storage = world
                .portal_poi
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            poi_storage.get_in_square(target_pos, search_radius, Some(poi::POI_TYPE_NETHER_PORTAL))
        };

        let mut best: Option<(PortalSearchResult, f64, i32)> = None;

        for pos in portal_positions {
            if pos.0.y < min_y || pos.0.y > search_max_y {
                continue;
            }

            if !worldborder.contains_block(pos.0.x, pos.0.z) {
                continue;
            }

            let (block, state) = world.get_block_and_state(&pos);
            if block != &Block::NETHER_PORTAL {
                continue;
            }
            let props = NetherPortalLikeProperties::from_state_id(state.id);
            let axis = props.axis;

            if let Some(portal) = Self::get_on_axis(world, &pos, axis)
                && portal.was_already_valid()
            {
                // Use POI position for distance calculation (matches vanilla behavior)
                let dist = f64::from(target_pos.0.squared_distance_to(pos.0.x, pos.0.y, pos.0.z));
                let y = portal.lower_conor.0.y;

                let is_better = match &best {
                    None => true,
                    Some((_, best_dist, best_y)) => {
                        dist < *best_dist
                            || ((dist - *best_dist).abs() < f64::EPSILON && y < *best_y)
                    }
                };

                if is_better {
                    best = Some((
                        PortalSearchResult {
                            lower_corner: portal.lower_conor,
                            axis: portal.axis,
                            width: portal.width,
                            height: portal.height,
                        },
                        dist,
                        y,
                    ));
                }
            }
        }

        best.map(|(result, _, _)| result)
    }

    pub fn spiral_around(center_x: i32, center_z: i32, radius: i32) -> SpiralIterator {
        SpiralIterator::new(center_x, center_z, radius)
    }

    #[allow(clippy::too_many_lines)]
    pub fn find_safe_location(
        world: &Arc<World>,
        target_pos: BlockPos,
        axis: HorizontalAxis,
    ) -> Option<(BlockPos, HorizontalAxis, bool)> {
        tracing::debug!(
            "[PORTAL-FIND-SAFE] Finding safe location in {:?} around target {:?}",
            world.dimension.minecraft_name,
            target_pos
        );
        let min_y = world.min_y;
        let max_y = min_y + world.dimension.height - 1;
        let worldborder = world
            .worldborder
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        let max_placeable_y = if world.dimension.has_ceiling {
            (min_y + world.dimension.logical_height - 1).min(max_y)
        } else {
            max_y
        };

        let direction = if axis == HorizontalAxis::X {
            BlockDirection::East
        } else {
            BlockDirection::South // Positive Z direction
        };
        let perpendicular = if axis == HorizontalAxis::X {
            BlockDirection::South
        } else {
            BlockDirection::West
        };

        let mut closest_full_dist_sq: f64 = -1.0;
        let mut closest_full_pos: Option<BlockPos> = None;
        let mut closest_partial_dist_sq: f64 = -1.0;
        let mut closest_partial_pos: Option<BlockPos> = None;

        for (check_x, check_z) in SpiralIterator::new(target_pos.0.x, target_pos.0.z, 16) {
            let column_pos = BlockPos::new(check_x, 0, check_z);
            let next_pos = column_pos.offset_dir(direction.to_offset(), 1);

            if !worldborder.contains_block(column_pos.0.x, column_pos.0.z)
                || !worldborder.contains_block(next_pos.0.x, next_pos.0.z)
            {
                continue;
            }

            let height = world
                .get_heightmap_height(ChunkHeightmapType::MotionBlocking, check_x, check_z)
                .min(max_placeable_y);

            let mut y = height;
            while y >= min_y {
                let test_pos = BlockPos::new(check_x, y, check_z);
                let state = world.get_block_state(&test_pos);

                if Self::is_valid_portal_air(state) {
                    let first_empty_y = y;
                    while y > min_y {
                        let below_pos = BlockPos::new(check_x, y - 1, check_z);
                        let below_state = world.get_block_state(&below_pos);
                        if !Self::is_valid_portal_air(below_state) {
                            break;
                        }
                        y -= 1;
                    }

                    if y + 4 <= max_placeable_y {
                        let delta_y = first_empty_y - y;
                        if delta_y <= 0 || delta_y >= 3 {
                            let candidate_pos = BlockPos::new(check_x, y, check_z);
                            if Self::can_host_frame(world, candidate_pos, direction, perpendicular, 0) {
                                let distance = f64::from(target_pos.0.squared_distance_to(
                                    candidate_pos.0.x,
                                    candidate_pos.0.y,
                                    candidate_pos.0.z,
                                ));

                                if Self::can_host_frame(world, candidate_pos, direction, perpendicular, -1)
                                    && Self::can_host_frame(world, candidate_pos, direction, perpendicular, 1)
                                    && (closest_full_dist_sq < 0.0 || distance < closest_full_dist_sq)
                                {
                                    closest_full_dist_sq = distance;
                                    closest_full_pos = Some(candidate_pos);
                                }

                                if closest_full_dist_sq < 0.0
                                    && (closest_partial_dist_sq < 0.0 || distance < closest_partial_dist_sq)
                                {
                                    closest_partial_dist_sq = distance;
                                    closest_partial_pos = Some(candidate_pos);
                                }
                            }
                        }
                    }
                }
                y -= 1;
            }
        }

        if closest_full_dist_sq < 0.0 && closest_partial_dist_sq >= 0.0 {
            closest_full_pos = closest_partial_pos;
            closest_full_dist_sq = closest_partial_dist_sq;
        }

        if let Some(pos) = closest_full_pos {
            tracing::debug!(
                "[PORTAL-FIND-SAFE] Found candidate location at {:?} (axis: {:?}, dist: {:.1})",
                pos,
                axis,
                closest_full_dist_sq.sqrt()
            );
            return Some((pos, axis, false));
        }

        // Vanilla safe underground/enclosed fallback:
        // clamp Y between max(min_y + 1, 70) and max_placeable_y - 9
        let min_start_y = (min_y + 1).max(70);
        let max_start_y = max_placeable_y - 9;
        if max_start_y < min_start_y {
            tracing::warn!(
                "[PORTAL-FIND-SAFE] Height limit too constrained for fallback: min {} max {}",
                min_start_y,
                max_start_y
            );
            return None;
        }

        let fallback_y = target_pos.0.y.clamp(min_start_y, max_start_y);
        let fallback_pos = BlockPos::new(
            target_pos.0.x - direction.to_offset().x,
            fallback_y,
            target_pos.0.z - direction.to_offset().z,
        );
        let clamped = worldborder.clamp_block(fallback_pos.0.x, fallback_pos.0.z);
        let final_pos = BlockPos::new(clamped.0, fallback_y, clamped.1);

        tracing::debug!(
            "[PORTAL-FIND-SAFE] No natural clearance found; using enclosed fallback at {:?} (axis: {:?})",
            final_pos,
            axis
        );
        Some((final_pos, axis, true))
    }

    const fn is_valid_portal_air(state: &BlockState) -> bool {
        state.replaceable() && !state.is_liquid()
    }

    fn can_host_frame(
        world: &World,
        origin: BlockPos,
        direction: BlockDirection,
        perpendicular: BlockDirection,
        offset: i32,
    ) -> bool {
        for width in -1..3 {
            for height in -1..4 {
                let pos = origin
                    .offset_dir(direction.to_offset(), width)
                    .offset_dir(perpendicular.to_offset(), offset)
                    .offset_dir(BlockDirection::Up.to_offset(), height);

                let state = world.get_block_state(&pos);

                if height < 0 {
                    if !state.is_solid_block() {
                        return false;
                    }
                } else if !Self::is_valid_portal_air(state) {
                    return false;
                }
            }
        }
        true
    }

    pub fn build_portal_frame(
        world: &Arc<World>,
        lower_corner: BlockPos,
        axis: HorizontalAxis,
        is_fallback: bool,
    ) {
        let direction = if axis == HorizontalAxis::X {
            BlockDirection::East
        } else {
            BlockDirection::South // Fixed: positive Z direction
        };
        let perpendicular = if axis == HorizontalAxis::X {
            BlockDirection::South // Fixed: East.rotateYClockwise()
        } else {
            BlockDirection::West // Fixed: South.rotateYClockwise()
        };

        let obsidian_state = Block::OBSIDIAN.default_state.id;
        let air_state = Block::AIR.default_state.id;

        if is_fallback {
            // Clear area around the portal matching vanilla exactly:
            // perpendicular: -1, 0, 1 (3 blocks)
            // portal_dir: 0, 1 (2 blocks - portal interior only)
            // height: -1, 0, 1, 2 (4 blocks)
            for perp in -1..2 {
                for portal_dir in 0..2 {
                    for height in -1..3 {
                        let pos = lower_corner
                            .offset_dir(direction.to_offset(), portal_dir)
                            .offset_dir(perpendicular.to_offset(), perp)
                            .offset_dir(BlockDirection::Up.to_offset(), height);

                        let state = if height < 0 {
                            obsidian_state
                        } else {
                            air_state
                        };
                        world.set_block_state(&pos, state, BlockFlags::NOTIFY_ALL);
                    }
                }
            }
        }

        for portal_dir in -1..3 {
            for height in -1..4 {
                if portal_dir == -1 || portal_dir == 2 || height == -1 || height == 3 {
                    let pos = lower_corner
                        .offset_dir(direction.to_offset(), portal_dir)
                        .offset_dir(BlockDirection::Up.to_offset(), height);
                    world.set_block_state(&pos, obsidian_state, BlockFlags::NOTIFY_ALL);
                }
            }
        }

        let mut props = NetherPortalLikeProperties::default(&Block::NETHER_PORTAL);
        props.axis = axis;
        let portal_state = props.to_state_id(&Block::NETHER_PORTAL);

        for x in 0..2 {
            for y in 0..3 {
                let pos = lower_corner
                    .offset_dir(direction.to_offset(), x)
                    .offset_dir(BlockDirection::Up.to_offset(), y);
                world.set_block_state(
                    &pos,
                    portal_state,
                    BlockFlags::NOTIFY_LISTENERS | BlockFlags::FORCE_STATE,
                );
                world
                    .portal_poi
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .add_portal(pos);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pumpkin_util::math::boundingbox::EntityDimensions;

    #[test]
    fn portal_teleport_position_x_axis() {
        let portal = PortalSearchResult {
            lower_corner: BlockPos::new(10, 64, 5),
            axis: HorizontalAxis::X,
            width: 2,
            height: 3,
        };
        let pos = portal.get_teleport_position();
        // Width is 2 along X, so center X is 10 + 1.0 = 11.0, Y is 64.0, Z is 5.5
        assert_eq!(pos, Vector3::new(11.0, 64.0, 5.5));
    }

    #[test]
    fn portal_teleport_position_z_axis() {
        let portal = PortalSearchResult {
            lower_corner: BlockPos::new(5, 64, 10),
            axis: HorizontalAxis::Z,
            width: 2,
            height: 3,
        };
        let pos = portal.get_teleport_position();
        // Width is 2 along Z, so center X is 5.5, Y is 64.0, Z is 10 + 1.0 = 11.0
        assert_eq!(pos, Vector3::new(5.5, 64.0, 11.0));
    }

    #[test]
    fn portal_relative_exit_position() {
        let src_portal = PortalSearchResult {
            lower_corner: BlockPos::new(100, 64, 200),
            axis: HorizontalAxis::X,
            width: 2,
            height: 3,
        };
        let dest_portal = PortalSearchResult {
            lower_corner: BlockPos::new(12, 70, 25),
            axis: HorizontalAxis::X,
            width: 2,
            height: 3,
        };
        let dimensions = EntityDimensions {
            width: 0.6,
            height: 1.8,
            eye_height: 1.62,
        };

        // Player is at the center of the source portal
        let player_pos = src_portal.get_teleport_position();
        let rel_pos = src_portal.entity_pos_in_portal(player_pos, &dimensions);
        let exit_pos = dest_portal.calculate_exit_position(rel_pos, &dimensions);

        // Should exit near the center of the destination portal
        assert!((exit_pos.x - 13.0).abs() < 0.01);
        assert!((exit_pos.y - 70.0).abs() < 0.01);
        assert!((exit_pos.z - 25.5).abs() < 0.01);
    }

    #[test]
    fn portal_cross_axis_x_to_z() {
        let src_portal = PortalSearchResult {
            lower_corner: BlockPos::new(100, 64, 200),
            axis: HorizontalAxis::X,
            width: 2,
            height: 3,
        };
        let dest_portal = PortalSearchResult {
            lower_corner: BlockPos::new(12, 70, 25),
            axis: HorizontalAxis::Z,
            width: 2,
            height: 3,
        };
        let dimensions = EntityDimensions {
            width: 0.6,
            height: 1.8,
            eye_height: 1.62,
        };

        // Player is at the center of the source X portal
        let player_pos = src_portal.get_teleport_position();
        let rel_pos = src_portal.entity_pos_in_portal(player_pos, &dimensions);
        let exit_pos = dest_portal.calculate_exit_position(rel_pos, &dimensions);

        // In Z portal: X is perpendicular (12 + 0.5 = 12.5), Z is along portal (25 + 1.0 = 26.0)
        assert!((exit_pos.x - 12.5).abs() < 0.01);
        assert!((exit_pos.y - 70.0).abs() < 0.01);
        assert!((exit_pos.z - 26.0).abs() < 0.01);
    }

    #[test]
    fn portal_yaw_rotation() {
        let x_portal = PortalSearchResult {
            lower_corner: BlockPos::new(0, 64, 0),
            axis: HorizontalAxis::X,
            width: 2,
            height: 3,
        };
        let z_portal = PortalSearchResult {
            lower_corner: BlockPos::new(0, 64, 0),
            axis: HorizontalAxis::Z,
            width: 2,
            height: 3,
        };

        // Same axis -> no rotation
        assert_eq!(
            x_portal.calculate_teleport_yaw(45.0, Some(HorizontalAxis::X)),
            45.0
        );
        assert_eq!(
            z_portal.calculate_teleport_yaw(45.0, Some(HorizontalAxis::Z)),
            45.0
        );

        // Cross axis -> 90 degree rotation
        assert_eq!(
            z_portal.calculate_teleport_yaw(45.0, Some(HorizontalAxis::X)),
            135.0
        );
        assert_eq!(
            x_portal.calculate_teleport_yaw(45.0, Some(HorizontalAxis::Z)),
            -45.0
        );
    }

    #[test]
    fn spiral_iterator_sequence() {
        let coords: Vec<(i32, i32)> = NetherPortal::spiral_around(0, 0, 1).collect();
        // First coordinate must be center (0, 0)
        assert_eq!(coords[0], (0, 0));
        // Next is EAST (+1, 0)
        assert_eq!(coords[1], (1, 0));
        // Next is SOUTH (+1, +1)
        assert_eq!(coords[2], (1, 1));
        // Next are WEST (0, 1), (-1, 1)
        assert_eq!(coords[3], (0, 1));
        assert_eq!(coords[4], (-1, 1));
        // Next are NORTH (-1, 0), (-1, -1)
        assert_eq!(coords[5], (-1, 0));
        assert_eq!(coords[6], (-1, -1));
    }

    #[test]
    fn search_radius_nether_is_16() {
        assert_eq!(SEARCH_RADIUS_NETHER, 16);
        assert_eq!(SEARCH_RADIUS_OVERWORLD, 128);
    }
}

