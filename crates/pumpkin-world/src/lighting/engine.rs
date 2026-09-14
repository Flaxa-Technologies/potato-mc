use crate::chunk::format::LightContainer;
use crate::chunk_system::Chunk;
use crate::chunk_system::generation_cache::Cache;
use crate::generation::height_limit::HeightLimitView;
use crate::generation::proto_chunk::GenerationCache;
use crate::lighting::storage::{get_block_light, get_sky_light, set_block_light, set_sky_light};
use pumpkin_config::lighting::LightingEngineConfig;
use pumpkin_data::{BlockDirection, BlockState, BlockStateId};
use pumpkin_util::HeightMap;
use pumpkin_util::math::position::BlockPos;
use pumpkin_util::math::vector3::Vector3;
use std::collections::VecDeque;

use crate::ProtoChunk;

pub trait LightProvider {
    fn get_light(cache: &Cache, pos: BlockPos) -> u8;
    fn set_light(cache: &mut Cache, pos: BlockPos, level: u8);
    fn get_light_proto(
        chunk: &ProtoChunk,
        section_idx: usize,
        lx: usize,
        ly: usize,
        lz: usize,
    ) -> u8;
    fn set_light_proto(
        chunk: &mut ProtoChunk,
        section_idx: usize,
        lx: usize,
        ly: usize,
        lz: usize,
        level: u8,
    );
    fn propagate_level(current_level: u8, opacity: u8, dir: BlockDirection) -> u8;
}

pub struct BlockLightProvider;
impl LightProvider for BlockLightProvider {
    #[inline]
    fn get_light(cache: &Cache, pos: BlockPos) -> u8 {
        get_block_light(cache, pos)
    }
    #[inline]
    fn set_light(cache: &mut Cache, pos: BlockPos, level: u8) {
        set_block_light(cache, pos, level);
    }
    #[inline]
    fn get_light_proto(
        chunk: &ProtoChunk,
        section_idx: usize,
        lx: usize,
        ly: usize,
        lz: usize,
    ) -> u8 {
        chunk
            .light
            .block_light
            .get(section_idx)
            .map_or(0, |c| c.get(lx, ly, lz))
    }
    #[inline]
    fn set_light_proto(
        chunk: &mut ProtoChunk,
        section_idx: usize,
        lx: usize,
        ly: usize,
        lz: usize,
        level: u8,
    ) {
        if let Some(c) = chunk.light.block_light.get_mut(section_idx) {
            c.set(lx, ly, lz, level);
        }
    }
    #[inline]
    fn propagate_level(current_level: u8, opacity: u8, _dir: BlockDirection) -> u8 {
        current_level.saturating_sub(opacity.max(1))
    }
}

pub struct SkyLightProvider;
impl LightProvider for SkyLightProvider {
    #[inline]
    fn get_light(cache: &Cache, pos: BlockPos) -> u8 {
        get_sky_light(cache, pos)
    }
    #[inline]
    fn set_light(cache: &mut Cache, pos: BlockPos, level: u8) {
        set_sky_light(cache, pos, level);
    }
    #[inline]
    fn get_light_proto(
        chunk: &ProtoChunk,
        section_idx: usize,
        lx: usize,
        ly: usize,
        lz: usize,
    ) -> u8 {
        if let Some(c) = chunk.light.sky_light.get(section_idx) {
            match c {
                LightContainer::Full(data) => {
                    let index = ly * 16 * 16 + lz * 16 + lx;
                    (data[index >> 1] >> (4 * (index & 1))) & 0x0F
                }
                LightContainer::Empty(val) => {
                    if *val == 0 {
                        let ny = chunk.bottom_y() as i32 + (section_idx as i32 * 16) + ly as i32;
                        if ny >= chunk.top_block_height_exclusive(lx as i32, lz as i32) {
                            return 15;
                        }
                    }
                    *val
                }
            }
        } else {
            15
        }
    }
    #[inline]
    fn set_light_proto(
        chunk: &mut ProtoChunk,
        section_idx: usize,
        lx: usize,
        ly: usize,
        lz: usize,
        level: u8,
    ) {
        if let Some(c) = chunk.light.sky_light.get_mut(section_idx) {
            c.set(lx, ly, lz, level);
        }
    }
    #[inline]
    fn propagate_level(current_level: u8, opacity: u8, dir: BlockDirection) -> u8 {
        if current_level == 15 && dir == BlockDirection::Down && opacity == 0 {
            return 15;
        }

        current_level.saturating_sub(opacity.max(1))
    }
}

#[derive(Clone, Copy, Debug)]
pub struct PropagationEntry {
    pub lx: u8,
    pub ly: u16,
    pub lz: u8,
    pub level: u8,
    pub skip_dir: u8, // 0..5 for BlockDirection, 6 for None
}

pub struct VisitedBitSet {
    bits: Vec<u64>,
    pub min_x: i32,
    pub min_y: i32,
    pub min_z: i32,
    pub size_x: usize,
    pub size_y: usize,
    pub size_z: usize,
}

impl Default for VisitedBitSet {
    fn default() -> Self {
        Self::new()
    }
}

impl VisitedBitSet {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            bits: Vec::new(),
            min_x: 0,
            min_y: 0,
            min_z: 0,
            size_x: 0,
            size_y: 0,
            size_z: 0,
        }
    }

    pub fn ensure_capacity(
        &mut self,
        min_x: i32,
        min_y: i32,
        min_z: i32,
        size_x: usize,
        size_y: usize,
        size_z: usize,
    ) {
        self.min_x = min_x;
        self.min_y = min_y;
        self.min_z = min_z;
        self.size_x = size_x;
        self.size_y = size_y;
        self.size_z = size_z;
        let total = size_x * size_y * size_z;
        let words = total.div_ceil(64);
        if self.bits.len() == words {
            self.bits.fill(0);
        } else {
            self.bits.resize(words, 0);
        }
    }

    #[inline]
    pub fn clear(&mut self) {
        self.bits.fill(0);
    }

    #[inline]
    #[must_use]
    pub fn is_visited_idx(&self, idx: usize) -> bool {
        let word = idx >> 6;
        let mask = 1u64 << (idx & 63);
        unsafe {
            (*self.bits.get_unchecked(word) & mask) != 0
        }
    }

    #[inline]
    pub fn test_and_set_idx(&mut self, idx: usize) -> bool {
        let word = idx >> 6;
        let mask = 1u64 << (idx & 63);
        unsafe {
            let w = self.bits.get_unchecked_mut(word);
            let prev = *w;
            if prev & mask != 0 {
                return false;
            }
            *w = prev | mask;
            true
        }
    }

    #[inline]
    pub fn test_and_set(&mut self, x: i32, y: i32, z: i32) -> bool {
        let lx = x - self.min_x;
        let ly = y - self.min_y;
        let lz = z - self.min_z;
        if lx < 0 || ly < 0 || lz < 0 {
            return false;
        }
        let lx = lx as usize;
        let ly = ly as usize;
        let lz = lz as usize;
        if lx >= self.size_x || ly >= self.size_y || lz >= self.size_z {
            return false;
        }
        let idx = (ly * self.size_z + lz) * self.size_x + lx;
        self.test_and_set_idx(idx)
    }

    #[inline]
    #[must_use]
    pub fn is_visited(&self, x: i32, y: i32, z: i32) -> bool {
        let lx = x - self.min_x;
        let ly = y - self.min_y;
        let lz = z - self.min_z;
        if lx < 0 || ly < 0 || lz < 0 {
            return true;
        }
        let lx = lx as usize;
        let ly = ly as usize;
        let lz = lz as usize;
        if lx >= self.size_x || ly >= self.size_y || lz >= self.size_z {
            return true;
        }
        let idx = (ly * self.size_z + lz) * self.size_x + lx;
        self.is_visited_idx(idx)
    }
}

static OPACITY_TABLE: std::sync::LazyLock<Box<[u8]>> = std::sync::LazyLock::new(|| {
    let mut table = vec![0u8; 65536];
    for id in 0..=u16::MAX {
        if let Some(state_id) = BlockStateId::new(id) {
            if state_id == BlockStateId::AIR {
                table[id as usize] = 0;
            } else {
                let s = state_id.to_state();
                table[id as usize] = if s.can_occlude() {
                    s.opacity.max(1)
                } else {
                    s.opacity
                };
            }
        }
    }
    table.into_boxed_slice()
});

#[inline(always)]
pub fn get_effective_opacity(state_id: BlockStateId) -> u8 {
    let id = state_id.as_u16() as usize;
    unsafe { *OPACITY_TABLE.get_unchecked(id) }
}

pub struct LightPropagator<P: LightProvider> {
    pub(crate) queue: Vec<PropagationEntry>,
    pub(crate) head: usize,
    pub(crate) visited: VisitedBitSet,
    pub(crate) decrease_queue: VecDeque<(BlockPos, u8)>,
    _marker: std::marker::PhantomData<P>,
}

impl<P: LightProvider> LightPropagator<P> {
    #[must_use]
    pub fn new() -> Self {
        Self {
            queue: Vec::with_capacity(8192),
            head: 0,
            visited: VisitedBitSet::new(),
            decrease_queue: VecDeque::new(),
            _marker: std::marker::PhantomData,
        }
    }

    pub fn clear(&mut self) {
        self.queue.clear();
        self.head = 0;
        self.visited.clear();
        self.decrease_queue.clear();
    }

    #[inline]
    #[must_use]
    pub fn has_work(&self) -> bool {
        self.head < self.queue.len()
    }

    #[inline]
    pub fn push_pos(
        &mut self,
        x: i32,
        y: i32,
        z: i32,
        level: u8,
        skip_dir: Option<BlockDirection>,
    ) {
        let lx = x - self.visited.min_x;
        let ly = y - self.visited.min_y;
        let lz = z - self.visited.min_z;
        if lx >= 0
            && (lx as usize) < self.visited.size_x
            && ly >= 0
            && (ly as usize) < self.visited.size_y
            && lz >= 0
            && (lz as usize) < self.visited.size_z
        {
            let skip = match skip_dir {
                Some(d) => d as u8,
                None => 6,
            };
            self.queue.push(PropagationEntry {
                lx: lx as u8,
                ly: ly as u16,
                lz: lz as u8,
                level,
                skip_dir: skip,
            });
        }
    }

    /// Flood-fills light outward from every entry currently queued.
    ///
    /// PERF NOTE (fixed): this function used to leave `self.queue`/`self.head`
    /// untouched after the drain loop finished. That's harmless for the
    /// generation-time callers (`propagate_light` / `convert_light`), which
    /// call `self.clear()` themselves before and after use — but
    /// `LightEngine::run_light_updates` / `process_decrease_queue` call this
    /// directly on a long-lived propagator (one per loaded chunk/region,
    /// invoked on every block place/break) and never cleared it. Since `head`
    /// only ever increased and the Vec was never truncated, the queue's
    /// length grew without bound for the lifetime of the server region,
    /// costing memory and cache locality over time. Clearing here is always
    /// safe: by the time the while-loop below exits, `head == queue.len()`
    /// in every case, so resetting to empty changes no observable behavior
    /// for any caller, and `Vec::clear()` keeps the allocation so there's no
    /// added reallocation cost on the next call.
    ///
    /// PERF NOTE (optimized): per queue entry, the 6-direction expansion used
    /// to recompute `rel_x`/`rel_z`/`chunk_idx` and the full
    /// `(nly*size_z+nlz)*size_x+nlx` multiply-based index independently for
    /// every direction. Since a given entry's vertical neighbors (up/down)
    /// never change `lx`/`lz`, and each horizontal neighbor only ever moves
    /// one axis by exactly one block, all of that is now precomputed once
    /// per entry (`base_offset`, `ly_stride`, `base_chunk_idx`, etc.) and
    /// each direction derives its neighbor index with a single add/sub
    /// instead of two multiplies, and only recomputes `chunk_idx` when a
    /// direction actually crosses a 16-block chunk boundary. All subtractions
    /// are guarded by the same boundary checks the original code already had
    /// before touching that axis, so there is no new underflow risk.
    #[expect(clippy::too_many_lines)]
    pub fn propagate(&mut self, cache: &mut Cache) {
        let cache_size = cache.size as usize;
        let min_y = cache.bottom_y() as i32;
        let max_y = min_y + cache.height() as i32;

        let min_x = cache.x * 16;
        let min_z = cache.z * 16;
        let size_x = cache_size * 16;
        let size_z = cache_size * 16;
        let size_y = (max_y - min_y) as usize;
        let stride_y = size_z * size_x;

        while self.head < self.queue.len() {
            let entry = self.queue[self.head];
            self.head += 1;

            let current_light = entry.level;
            if current_light <= 1 {
                continue;
            }

            let lx = entry.lx as usize;
            let ly = entry.ly as usize;
            let lz = entry.lz as usize;
            let skip_dir = entry.skip_dir;

            // Precomputed once per entry; directions that don't move the
            // corresponding axis reuse these instead of recomputing
            // shifts/multiplies for all 6 directions.
            let base_rel_x = lx >> 4;
            let base_rel_z = lz >> 4;
            let base_local_x = lx & 15;
            let base_local_z = lz & 15;
            let base_chunk_idx = base_rel_x * cache_size + base_rel_z;
            let base_section_idx = ly >> 4;
            let base_local_y = ly & 15;
            let ly_stride = ly * stride_y;
            let base_offset = lz * size_x + lx;

            // 6 directions:
            // 0: Down, 1: Up, 2: North (-Z), 3: South (+Z), 4: West (-X), 5: East (+X)
            for dir_idx in 0..6u8 {
                if dir_idx == skip_dir {
                    continue;
                }

                let (
                    nlx,
                    nly,
                    nlz,
                    max_possible,
                    block_dir,
                    n_idx,
                    chunk_idx,
                    local_x,
                    local_y,
                    local_z,
                    section_idx,
                ) = match dir_idx {
                    0 => {
                        if ly == 0 {
                            continue;
                        }
                        let max_p = if current_light == 15 { 15 } else { current_light - 1 };
                        let nly = ly - 1;
                        (
                            lx,
                            nly,
                            lz,
                            max_p,
                            BlockDirection::Down,
                            ly_stride - stride_y + base_offset,
                            base_chunk_idx,
                            base_local_x,
                            nly & 15,
                            base_local_z,
                            nly >> 4,
                        )
                    }
                    1 => {
                        if ly + 1 >= size_y {
                            continue;
                        }
                        let nly = ly + 1;
                        (
                            lx,
                            nly,
                            lz,
                            current_light - 1,
                            BlockDirection::Up,
                            ly_stride + stride_y + base_offset,
                            base_chunk_idx,
                            base_local_x,
                            nly & 15,
                            base_local_z,
                            nly >> 4,
                        )
                    }
                    2 => {
                        if lz == 0 {
                            continue;
                        }
                        let nlz = lz - 1;
                        let rel_z = nlz >> 4;
                        let local_z = nlz & 15;
                        let chunk_idx = if rel_z == base_rel_z {
                            base_chunk_idx
                        } else {
                            base_rel_x * cache_size + rel_z
                        };
                        (
                            lx,
                            ly,
                            nlz,
                            current_light - 1,
                            BlockDirection::North,
                            ly_stride + base_offset - size_x,
                            chunk_idx,
                            base_local_x,
                            base_local_y,
                            local_z,
                            base_section_idx,
                        )
                    }
                    3 => {
                        if lz + 1 >= size_z {
                            continue;
                        }
                        let nlz = lz + 1;
                        let rel_z = nlz >> 4;
                        let local_z = nlz & 15;
                        let chunk_idx = if rel_z == base_rel_z {
                            base_chunk_idx
                        } else {
                            base_rel_x * cache_size + rel_z
                        };
                        (
                            lx,
                            ly,
                            nlz,
                            current_light - 1,
                            BlockDirection::South,
                            ly_stride + base_offset + size_x,
                            chunk_idx,
                            base_local_x,
                            base_local_y,
                            local_z,
                            base_section_idx,
                        )
                    }
                    4 => {
                        if lx == 0 {
                            continue;
                        }
                        let nlx = lx - 1;
                        let rel_x = nlx >> 4;
                        let local_x = nlx & 15;
                        let chunk_idx = if rel_x == base_rel_x {
                            base_chunk_idx
                        } else {
                            rel_x * cache_size + base_rel_z
                        };
                        (
                            nlx,
                            ly,
                            lz,
                            current_light - 1,
                            BlockDirection::West,
                            ly_stride + base_offset - 1,
                            chunk_idx,
                            local_x,
                            base_local_y,
                            base_local_z,
                            base_section_idx,
                        )
                    }
                    5 => {
                        if lx + 1 >= size_x {
                            continue;
                        }
                        let nlx = lx + 1;
                        let rel_x = nlx >> 4;
                        let local_x = nlx & 15;
                        let chunk_idx = if rel_x == base_rel_x {
                            base_chunk_idx
                        } else {
                            rel_x * cache_size + base_rel_z
                        };
                        (
                            nlx,
                            ly,
                            lz,
                            current_light - 1,
                            BlockDirection::East,
                            ly_stride + base_offset + 1,
                            chunk_idx,
                            local_x,
                            base_local_y,
                            base_local_z,
                            base_section_idx,
                        )
                    }
                    _ => unreachable!(),
                };

                if self.visited.is_visited_idx(n_idx) {
                    continue;
                }

                let local_y_proto = nly;

                let (opacity, neighbor_light) = match &cache.chunks[chunk_idx] {
                    Chunk::Proto(c) => {
                        let light = P::get_light_proto(c, section_idx, local_x, local_y, local_z);
                        if light >= max_possible {
                            continue;
                        }
                        let state_id =
                            c.get_block_state_raw(local_x as i32, local_y_proto as i32, local_z as i32);
                        let op = get_effective_opacity(state_id);
                        (op, light)
                    }
                    Chunk::Level(lvl) => {
                        let nx = min_x + nlx as i32;
                        let ny = min_y + nly as i32;
                        let nz = min_z + nlz as i32;
                        let neighbor_pos = BlockPos(Vector3::new(nx, ny, nz));
                        let light = P::get_light(cache, neighbor_pos);
                        if light >= max_possible {
                            continue;
                        }
                        let state_id = lvl
                            .section
                            .get_block_absolute_y(local_x, ny, local_z)
                            .unwrap_or(BlockStateId::AIR);
                        let op = get_effective_opacity(state_id);
                        (op, light)
                    }
                };

                if opacity >= 15 {
                    self.visited.test_and_set_idx(n_idx);
                    continue;
                }

                let new_level = P::propagate_level(current_light, opacity, block_dir);

                if new_level > neighbor_light {
                    match &mut cache.chunks[chunk_idx] {
                        Chunk::Proto(c) => {
                            P::set_light_proto(
                                c,
                                section_idx,
                                local_x,
                                local_y,
                                local_z,
                                new_level,
                            );
                        }
                        Chunk::Level(_) => {
                            let nx = min_x + nlx as i32;
                            let ny = min_y + nly as i32;
                            let nz = min_z + nlz as i32;
                            let neighbor_pos = BlockPos(Vector3::new(nx, ny, nz));
                            P::set_light(cache, neighbor_pos, new_level);
                        }
                    }

                    if new_level > 1 && self.visited.test_and_set_idx(n_idx) {
                        self.queue.push(PropagationEntry {
                            lx: nlx as u8,
                            ly: nly as u16,
                            lz: nlz as u8,
                            level: new_level,
                            skip_dir: dir_idx ^ 1,
                        });
                    }
                }
            }
        }

        // See PERF NOTE (fixed) above: head == queue.len() here in every
        // case, so this is always a no-op behaviorally and just prevents
        // unbounded growth across repeated calls on a long-lived propagator.
        self.queue.clear();
        self.head = 0;
    }

    pub fn process_decrease_queue(&mut self, cache: &mut Cache) {
        let cache_x = cache.x;
        let cache_z = cache.z;
        let cache_size = cache.size;

        while let Some((pos, old_val)) = self.decrease_queue.pop_front() {
            for dir in BlockDirection::all() {
                let neighbor_pos = pos.offset(dir.to_offset());

                let (cx, _rel) = neighbor_pos.chunk_and_chunk_relative_position();
                let rel_x = cx.x - cache_x;
                let rel_z = cx.y - cache_z;

                if rel_x < 0 || rel_x >= cache_size || rel_z < 0 || rel_z >= cache_size {
                    continue;
                }

                let neighbor_light = P::get_light(cache, neighbor_pos);
                if neighbor_light == 0 {
                    continue;
                }

                let state = cache.get_block_state(&neighbor_pos.0).to_state();
                let opacity = if state.can_occlude() {
                    state.opacity.max(1)
                } else {
                    state.opacity
                };

                let predicted = P::propagate_level(old_val, opacity, dir);

                if neighbor_light == predicted || neighbor_light < old_val {
                    P::set_light(cache, neighbor_pos, 0);
                    self.decrease_queue
                        .push_back((neighbor_pos, neighbor_light));
                } else if neighbor_light >= old_val {
                    let nx = neighbor_pos.0.x;
                    let ny = neighbor_pos.0.y;
                    let nz = neighbor_pos.0.z;
                    // PERF NOTE (fixed): previously called push_pos()
                    // unconditionally and marked visited afterward, so
                    // multiple decreasing neighbors converging on the same
                    // still-lit block could queue it more than once (each
                    // duplicate wastefully re-running a full 6-direction
                    // expansion; harmless to correctness since propagation
                    // is idempotent, but wasted work). Now only queues it
                    // the first time it's actually marked visited.
                    if self.visited.test_and_set(nx, ny, nz) {
                        self.push_pos(nx, ny, nz, neighbor_light, None);
                    }
                }
            }
        }

        self.propagate(cache);
    }
}

pub type BlockLightPropagator = LightPropagator<BlockLightProvider>;
pub type SkyLightPropagator = LightPropagator<SkyLightProvider>;

impl<P: LightProvider> Default for LightPropagator<P> {
    fn default() -> Self {
        Self::new()
    }
}

impl BlockLightPropagator {
    pub fn propagate_light(&mut self, cache: &mut Cache) {
        self.clear();

        let min_y = cache.bottom_y() as i32;
        let max_y = min_y + cache.height() as i32;
        let center_x = cache.x + (cache.size / 2);
        let center_z = cache.z + (cache.size / 2);

        let start_x = center_x * 16 - 1;
        let start_z = center_z * 16 - 1;
        let end_x = start_x + 18;
        let end_z = start_z + 18;

        let min_x = cache.x * 16;
        let min_z = cache.z * 16;
        let size_x = (cache.size * 16) as usize;
        let size_z = (cache.size * 16) as usize;
        let size_y = (max_y - min_y) as usize;

        // Fast scan: check if any chunk overlapping the 18x18 region has emissive blocks
        let mut has_emissive = false;
        for rel_z in 0..cache.size {
            let chunk_world_z = (cache.z + rel_z) * 16;
            if chunk_world_z + 15 < start_z || chunk_world_z >= end_z {
                continue;
            }
            for rel_x in 0..cache.size {
                let chunk_world_x = (cache.x + rel_x) * 16;
                if chunk_world_x + 15 < start_x || chunk_world_x >= end_x {
                    continue;
                }
                let chunk_idx = (rel_x * cache.size + rel_z) as usize;
                match &cache.chunks[chunk_idx] {
                    Chunk::Proto(c) => {
                        if c.emissive_sections != 0 {
                            has_emissive = true;
                            break;
                        }
                    }
                    Chunk::Level(_) => {
                        has_emissive = true;
                        break;
                    }
                }
            }
            if has_emissive {
                break;
            }
        }

        if !has_emissive {
            return;
        }

        self.visited
            .ensure_capacity(min_x, min_y, min_z, size_x, size_y, size_z);

        for z in start_z..end_z {
            let rel_z = (z >> 4) - cache.z;
            let local_z = (z & 15) as usize;

            for x in start_x..end_x {
                let rel_x = (x >> 4) - cache.x;
                if rel_x < 0 || rel_x >= cache.size || rel_z < 0 || rel_z >= cache.size {
                    continue;
                }
                let chunk_idx = (rel_x * cache.size + rel_z) as usize;
                let local_x = (x & 15) as usize;

                match &mut cache.chunks[chunk_idx] {
                    Chunk::Proto(c) => {
                        let mut emissive = c.emissive_sections;
                        while emissive != 0 {
                            let sec = emissive.trailing_zeros() as usize;
                            emissive &= emissive - 1;
                            let sec_y_base = min_y + (sec as i32 * 16);
                            let local_y_base = sec as i32 * 16;
                            for dy in 0..16 {
                                let y = sec_y_base + dy;
                                let local_y_proto = local_y_base + dy;
                                let state_id = c.get_block_state_raw(
                                    local_x as i32,
                                    local_y_proto,
                                    local_z as i32,
                                );
                                if state_id == BlockStateId::AIR {
                                    continue;
                                }
                                let emission = state_id.to_state().luminance;
                                if emission > 0 {
                                    let local_y = dy as usize;
                                    if sec < c.light.block_light.len() {
                                        c.light.block_light[sec]
                                            .set(local_x, local_y, local_z, emission);
                                    }
                                    let lx_cache = (x - min_x) as usize;
                                    let lz_cache = (z - min_z) as usize;
                                    let ly_cache = (y - min_y) as usize;
                                    let idx = (ly_cache * size_z + lz_cache) * size_x + lx_cache;
                                    if self.visited.test_and_set_idx(idx) {
                                        self.queue.push(PropagationEntry {
                                            lx: lx_cache as u8,
                                            ly: ly_cache as u16,
                                            lz: lz_cache as u8,
                                            level: emission,
                                            skip_dir: 6,
                                        });
                                    }
                                }
                            }
                        }
                    }
                    Chunk::Level(lvl) => {
                        let mut light_engine = lvl
                            .light_engine
                            .lock()
                            .unwrap_or_else(std::sync::PoisonError::into_inner);
                        for y in min_y..max_y {
                            let state_id = lvl
                                .section
                                .get_block_absolute_y(local_x, y, local_z)
                                .unwrap_or(BlockStateId::AIR);
                            if state_id == BlockStateId::AIR {
                                continue;
                            }
                            let emission = state_id.to_state().luminance;
                            if emission > 0 {
                                let section_idx = ((y - min_y) >> 4) as usize;
                                let local_y = (y & 15) as usize;
                                if section_idx < light_engine.block_light.len() {
                                    light_engine.block_light[section_idx]
                                        .set(local_x, local_y, local_z, emission);
                                }
                                let lx_cache = (x - min_x) as usize;
                                let lz_cache = (z - min_z) as usize;
                                let ly_cache = (y - min_y) as usize;
                                let idx = (ly_cache * size_z + lz_cache) * size_x + lx_cache;
                                if self.visited.test_and_set_idx(idx) {
                                    self.queue.push(PropagationEntry {
                                        lx: lx_cache as u8,
                                        ly: ly_cache as u16,
                                        lz: lz_cache as u8,
                                        level: emission,
                                        skip_dir: 6,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        if !self.has_work() {
            return;
        }

        self.propagate(cache);
    }
}

impl SkyLightPropagator {
    #[expect(clippy::too_many_lines)]
    pub fn convert_light(&mut self, cache: &mut Cache) {
        self.clear();

        let center_x = cache.x + (cache.size / 2);
        let center_z = cache.z + (cache.size / 2);
        let start_x = center_x * 16 - 1;
        let start_z = center_z * 16 - 1;
        let end_x = start_x + 18;
        let end_z = start_z + 18;

        let bottom_y = cache.bottom_y() as i32;
        let max_y = bottom_y + cache.height() as i32;

        let min_x = cache.x * 16;
        let min_z = cache.z * 16;
        let size_x = (cache.size * 16) as usize;
        let size_z = (cache.size * 16) as usize;
        let size_y = (max_y - bottom_y) as usize;
        self.visited
            .ensure_capacity(min_x, bottom_y, min_z, size_x, size_y, size_z);

        let mut surface_heights = [0i32; 18 * 18];
        let mut max_center_top_y = bottom_y;

        for z in start_z..end_z {
            let lz = (z - start_z) as usize;
            for x in start_x..end_x {
                let lx = (x - start_x) as usize;
                let top_y = cache.get_top_y(&HeightMap::WorldSurface, x, z);
                surface_heights[lx * 18 + lz] = top_y;
                if (1..=16).contains(&lx) && (1..=16).contains(&lz) {
                    max_center_top_y = max_center_top_y.max(top_y);
                }
            }
        }

        let center_rel = cache.size / 2;
        let center_chunk_idx = (center_rel * cache.size + center_rel) as usize;
        let max_center_top_local_y = (max_center_top_y + 1 - bottom_y).max(0) as usize;
        let max_center_sec = max_center_top_local_y >> 4;

        if let Chunk::Proto(c) = &mut cache.chunks[center_chunk_idx] {
            let total_sections = c.light.sky_light.len();
            for sec in (max_center_sec + 1)..total_sections {
                c.light.sky_light[sec] = LightContainer::new_empty(15);
            }
        }

        match &mut cache.chunks[center_chunk_idx] {
            Chunk::Proto(c) => {
                for local_z in 0..16usize {
                    let lz = local_z + 1;
                    for local_x in 0..16usize {
                        let lx = local_x + 1;
                        let top_y = surface_heights[lx * 18 + lz];

                        let top_local_y = (top_y + 1 - bottom_y).max(0) as usize;
                        let top_sec = top_local_y >> 4;
                        let top_rem = top_local_y & 15;
                        if top_sec < c.light.sky_light.len() {
                            c.light.sky_light[top_sec]
                                .set_column_y_range(local_x, local_z, top_rem, 16, 15);
                            let fill_limit = (max_center_sec + 1).min(c.light.sky_light.len());
                            for sec in (top_sec + 1)..fill_limit {
                                c.light.sky_light[sec]
                                    .set_column_y_range(local_x, local_z, 0, 16, 15);
                            }
                        }

                        let mut light: i32 = 15;
                        for y in (bottom_y..=top_y).rev() {
                            let local_y_proto = y - bottom_y;
                            let state_id = c.get_block_state_raw(
                                local_x as i32,
                                local_y_proto,
                                local_z as i32,
                            );
                            let opacity = get_effective_opacity(state_id) as i32;

                            light = light.saturating_sub(opacity);
                            let light_val = if light <= 0 { 0 } else { light as u8 };
                            let section_idx = (local_y_proto >> 4) as usize;
                            let local_y = (y & 15) as usize;

                            if section_idx < c.light.sky_light.len() {
                                c.light.sky_light[section_idx]
                                    .set(local_x, local_y, local_z, light_val);
                            }

                            if light <= 0 {
                                break;
                            }
                        }
                    }
                }
            }
            Chunk::Level(c) => {
                let mut light_engine = c
                    .light_engine
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);

                for local_z in 0..16usize {
                    let lz = local_z + 1;
                    for local_x in 0..16usize {
                        let lx = local_x + 1;
                        let top_y = surface_heights[lx * 18 + lz];

                        for y in (top_y + 1)..max_y {
                            let section_idx = ((y - bottom_y) >> 4) as usize;
                            let local_y = (y & 15) as usize;
                            if section_idx < light_engine.sky_light.len() {
                                light_engine.sky_light[section_idx]
                                    .set(local_x, local_y, local_z, 15);
                            }
                        }

                        let mut light: i32 = 15;
                        for y in (bottom_y..=top_y).rev() {
                            let section_idx = ((y - bottom_y) >> 4) as usize;
                            let local_y = (y & 15) as usize;

                            let state_id = c
                                .section
                                .get_block_absolute_y(local_x, y, local_z)
                                .unwrap_or(BlockStateId::AIR);
                            let opacity = get_effective_opacity(state_id) as i32;

                            light = light.saturating_sub(opacity);
                            let light_val = if light <= 0 { 0 } else { light as u8 };

                            if section_idx < light_engine.sky_light.len() {
                                light_engine.sky_light[section_idx]
                                    .set(local_x, local_y, local_z, light_val);
                            }

                            if light <= 0 {
                                break;
                            }
                        }
                    }
                }
            }
        }

        for z in start_z..end_z {
            let lz = (z - start_z) as usize;
            let rel_z = (z >> 4) - cache.z;
            let local_z = (z & 15) as usize;

            for x in start_x..end_x {
                let lx = (x - start_x) as usize;
                let rel_x = (x >> 4) - cache.x;
                let local_x = (x & 15) as usize;

                let top_y = surface_heights[lx * 18 + lz];

                let north_top = if lz > 0 {
                    surface_heights[lx * 18 + (lz - 1)]
                } else {
                    top_y
                };
                let south_top = if lz + 1 < 18 {
                    surface_heights[lx * 18 + (lz + 1)]
                } else {
                    top_y
                };
                let west_top = if lx > 0 {
                    surface_heights[(lx - 1) * 18 + lz]
                } else {
                    top_y
                };
                let east_top = if lx + 1 < 18 {
                    surface_heights[(lx + 1) * 18 + lz]
                } else {
                    top_y
                };

                let max_neighbor_top = north_top.max(south_top).max(west_top).max(east_top);

                let lx_cache = x - min_x;
                let lz_cache = z - min_z;
                if lx_cache < 0
                    || lx_cache >= size_x as i32
                    || lz_cache < 0
                    || lz_cache >= size_z as i32
                {
                    continue;
                }
                let lx_cache = lx_cache as usize;
                let lz_cache = lz_cache as usize;

                // 1. Blocks strictly above top_y: only queue if strictly below a neighbor top.
                // For all blocks above top_y in this column, sky light is always 15.
                let start_y = (top_y + 1).max(bottom_y);
                let end_y = max_neighbor_top.min(max_y);
                if end_y > start_y {
                    for y in start_y..end_y {
                        let ly_cache = (y - bottom_y) as usize;
                        let idx = (ly_cache * size_z + lz_cache) * size_x + lx_cache;
                        if (y < north_top || y < south_top || y < west_top || y < east_top)
                            && self.visited.test_and_set_idx(idx)
                        {
                            self.queue.push(PropagationEntry {
                                lx: lx_cache as u8,
                                ly: ly_cache as u16,
                                lz: lz_cache as u8,
                                level: 15,
                                skip_dir: BlockDirection::Up as u8,
                            });
                        }
                    }
                }

                // 2. Blocks at and below surface: stop as soon as sky light reaches 0.
                let check_top_y = top_y.min(max_y - 1);
                if check_top_y >= bottom_y
                    && rel_x >= 0
                    && rel_x < cache.size
                    && rel_z >= 0
                    && rel_z < cache.size
                {
                    let chunk_idx = (rel_x * cache.size + rel_z) as usize;
                    match &cache.chunks[chunk_idx] {
                        Chunk::Proto(c) => {
                            for y in (bottom_y..=check_top_y).rev() {
                                let section_idx = ((y - bottom_y) >> 4) as usize;
                                let local_y = (y & 15) as usize;
                                let light = SkyLightProvider::get_light_proto(
                                    c,
                                    section_idx,
                                    local_x,
                                    local_y,
                                    local_z,
                                );
                                if light == 0 {
                                    break;
                                }

                                let is_at_surface = y == top_y;
                                let below_neighbor =
                                    y < north_top || y < south_top || y < west_top || y < east_top;

                                let ly_cache = (y - bottom_y) as usize;
                                let idx = (ly_cache * size_z + lz_cache) * size_x + lx_cache;

                                if (is_at_surface || below_neighbor)
                                    && self.visited.test_and_set_idx(idx)
                                {
                                    let skip_dir =
                                        if y >= top_y { BlockDirection::Up as u8 } else { 6 };
                                    self.queue.push(PropagationEntry {
                                        lx: lx_cache as u8,
                                        ly: ly_cache as u16,
                                        lz: lz_cache as u8,
                                        level: light,
                                        skip_dir,
                                    });
                                }
                            }
                        }
                        Chunk::Level(_) => {
                            for y in (bottom_y..=check_top_y).rev() {
                                let pos = BlockPos(Vector3::new(x, y, z));
                                let light = get_sky_light(cache, pos);
                                if light == 0 {
                                    break;
                                }

                                let is_at_surface = y == top_y;
                                let below_neighbor =
                                    y < north_top || y < south_top || y < west_top || y < east_top;

                                let ly_cache = (y - bottom_y) as usize;
                                let idx = (ly_cache * size_z + lz_cache) * size_x + lx_cache;

                                if (is_at_surface || below_neighbor)
                                    && self.visited.test_and_set_idx(idx)
                                {
                                    let skip_dir =
                                        if y >= top_y { BlockDirection::Up as u8 } else { 6 };
                                    self.queue.push(PropagationEntry {
                                        lx: lx_cache as u8,
                                        ly: ly_cache as u16,
                                        lz: lz_cache as u8,
                                        level: light,
                                        skip_dir,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        if !self.has_work() {
            return;
        }

        self.propagate(cache);
    }
}

pub struct LightEngine {
    block_light: BlockLightPropagator,
    sky_light: SkyLightPropagator,
}

impl LightEngine {
    #[must_use]
    pub fn new() -> Self {
        Self {
            block_light: BlockLightPropagator::new(),
            sky_light: SkyLightPropagator::new(),
        }
    }

    pub fn initialize_light(
        &mut self,
        cache: &mut Cache,
        config: &LightingEngineConfig,
        has_skylight: bool,
    ) {
        if *config != LightingEngineConfig::Default {
            return;
        }

        let should_skip = {
            let center_chunk = cache.get_center_chunk();
            center_chunk.stage >= crate::chunk_system::chunk_state::StagedChunkEnum::Lighting
        };
        if should_skip {
            return;
        }

        if has_skylight {
            self.sky_light.convert_light(cache);
        }
        self.block_light.propagate_light(cache);

        self.block_light.clear();
        self.sky_light.clear();
    }

    pub fn update_block_light(
        &mut self,
        cache: &mut Cache,
        pos: BlockPos,
        old_luminance: u8,
        new_luminance: u8,
    ) {
        if old_luminance > new_luminance {
            let current_light = get_block_light(cache, pos);
            if current_light > 0 {
                self.block_light
                    .decrease_queue
                    .push_back((pos, current_light));
                set_block_light(cache, pos, 0);
            }
        }

        if new_luminance > 0 {
            set_block_light(cache, pos, new_luminance);
            if self
                .block_light
                .visited
                .test_and_set(pos.0.x, pos.0.y, pos.0.z)
            {
                self.block_light.push_pos(pos.0.x, pos.0.y, pos.0.z, new_luminance, None);
            }
        }
    }

    pub fn run_light_updates(&mut self, cache: &mut Cache) {
        if !self.block_light.decrease_queue.is_empty() {
            self.block_light.process_decrease_queue(cache);
        }
        if self.block_light.has_work() {
            self.block_light.propagate(cache);
            self.block_light.visited.clear();
        }
        if !self.sky_light.decrease_queue.is_empty() {
            self.sky_light.process_decrease_queue(cache);
        }
        if self.sky_light.has_work() {
            self.sky_light.propagate(cache);
            self.sky_light.visited.clear();
        }
    }

    /// Checks if a block state has an empty shape for light occlusion, matching vanilla `LightEngine.isEmptyShape`.
    #[inline]
    #[must_use]
    pub const fn is_empty_shape(state: &BlockState) -> bool {
        !state.can_occlude()
    }

    /// Checks if two block states have different light properties, matching vanilla `LightEngine.hasDifferentLightProperties`.
    #[inline]
    #[must_use]
    pub fn has_different_light_properties(old_state: &BlockState, new_state: &BlockState) -> bool {
        if std::ptr::eq(old_state, new_state) || old_state.id == new_state.id {
            return false;
        }

        old_state.opacity != new_state.opacity
            || old_state.luminance != new_state.luminance
            || old_state.can_occlude() != new_state.can_occlude()
    }

    /// Returns the minimum light dampening for this block state, matching vanilla `LightEngine.getOpacity`.
    #[inline]
    #[must_use]
    pub const fn get_opacity(state: &BlockState) -> u8 {
        if state.opacity > 1 { state.opacity } else { 1 }
    }

    /// Calculates the light dampening between two blocks, matching vanilla `LightEngine.getLightDampeningInto`.
    #[inline]
    #[must_use]
    pub const fn get_light_dampening_into(
        from_state: &BlockState,
        to_state: &BlockState,
        _direction: BlockDirection,
        simple_opacity: u8,
    ) -> u8 {
        let from_empty = Self::is_empty_shape(from_state);
        let to_empty = Self::is_empty_shape(to_state);
        if from_empty && to_empty {
            return simple_opacity;
        }
        if to_state.can_occlude() && to_state.is_solid_render() {
            return 16;
        }
        simple_opacity
    }
}

impl Default for LightEngine {
    fn default() -> Self {
        Self::new()
    }
}
