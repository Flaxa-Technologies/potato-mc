use crate::types::{Block, Location, RayTraceResult, Vector3};

/// Utility algorithms for line-of-sight raytracing against blocks and entities.
pub struct RayTrace;

impl RayTrace {
    /// Raytraces from an origin point along a direction vector to find intersecting blocks.
    /// Calls the provided `lookup_fn` to test whether a block at coordinates (x, y, z) is solid.
    pub fn trace_blocks<F>(
        start: Vector3,
        direction: Vector3,
        max_distance: f64,
        step: f64,
        mut lookup_fn: F,
    ) -> Option<(Block, Location, RayTraceResult)>
    where
        F: FnMut(i32, i32, i32) -> Option<Block>,
    {
        let dir = direction.normalize();
        let step_size = if step <= 0.0 { 0.2 } else { step };
        let mut current_dist = 0.0;

        let mut last_block_pos = (i32::MAX, i32::MAX, i32::MAX);

        while current_dist <= max_distance {
            let point = start.add(&dir.multiply(current_dist));
            let bx = point.x.floor() as i32;
            let by = point.y.floor() as i32;
            let bz = point.z.floor() as i32;

            if (bx, by, bz) != last_block_pos {
                last_block_pos = (bx, by, bz);
                if let Some(block) = lookup_fn(bx, by, bz) {
                    if block.block_type != "minecraft:air" && block.block_type != "air" {
                        let hit_pos = point;
                        let hit_loc = Location::new("", bx as f64, by as f64, bz as f64, 0.0, 0.0);
                        return Some((
                            block,
                            hit_loc,
                            RayTraceResult {
                                hit_position: hit_pos,
                                hit_normal: None,
                                distance: current_dist,
                            },
                        ));
                    }
                }
            }

            current_dist += step_size;
        }

        None
    }
}
