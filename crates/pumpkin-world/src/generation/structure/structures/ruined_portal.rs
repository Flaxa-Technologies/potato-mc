use std::sync::Arc;

use pumpkin_data::{Block, Mirror, Rotation, structures::StructureKeys};
use pumpkin_util::{
    HeightMap,
    math::{block_box::BlockBox, position::BlockPos, vector3::Vector3},
    random::{RandomGenerator, RandomImpl, hash_block_pos, legacy_rand::LegacyRand},
};

use crate::{
    ProtoChunk,
    generation::{
        positions::chunk_pos::{start_block_x, start_block_z},
        structure::{
            piece::StructurePieceType,
            structures::{
                StructureGenerator, StructureGeneratorContext, StructurePiece, StructurePieceBase,
                StructurePiecesCollector, StructurePosition, WorldPortalExt,
            },
            template::{
                BlockStateResolver, PaletteEntry, StructurePlaceSettings, StructureTemplate,
                get_template,
                processor::{
                    IgnoredBlock, PosRuleTest, ProcessorContext, ProcessorRule, RuleTest,
                    StructureProcessor,
                },
            },
        },
    },
};

const PORTALS: &[&str] = &[
    "ruined_portal/portal_1",
    "ruined_portal/portal_2",
    "ruined_portal/portal_3",
    "ruined_portal/portal_4",
    "ruined_portal/portal_5",
    "ruined_portal/portal_6",
    "ruined_portal/portal_7",
    "ruined_portal/portal_8",
    "ruined_portal/portal_9",
    "ruined_portal/portal_10",
];

const GIANT_PORTALS: &[&str] = &[
    "ruined_portal/giant_portal_1",
    "ruined_portal/giant_portal_2",
    "ruined_portal/giant_portal_3",
];

const NETHERRACK_PROBABILITY_BY_DISTANCE: &[f32] = &[
    1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.9, 0.9, 0.8, 0.7, 0.6, 0.4, 0.2,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerticalPlacement {
    OnLandSurface,
    PartlyBuried,
    OnOceanFloor,
    InMountain,
    Underground,
    InNether,
}

impl VerticalPlacement {
    #[must_use]
    pub const fn get_heightmap_type(self) -> HeightMap {
        match self {
            Self::OnOceanFloor => HeightMap::OceanFloorWg,
            _ => HeightMap::WorldSurfaceWg,
        }
    }
}

#[derive(Clone, Debug)]
pub struct RuinedPortalProperties {
    pub cold: bool,
    pub mossiness: f32,
    pub air_pocket: bool,
    pub overgrown: bool,
    pub vines: bool,
    pub replace_with_blackstone: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct RuinedPortalSetup {
    pub placement: VerticalPlacement,
    pub air_pocket_probability: f32,
    pub mossiness: f32,
    pub overgrown: bool,
    pub vines: bool,
    pub can_be_cold: bool,
    pub replace_with_blackstone: bool,
    pub weight: f32,
}

impl RuinedPortalSetup {
    #[must_use]
    pub const fn for_variant(variant: StructureKeys) -> &'static [Self] {
        match variant {
            StructureKeys::RuinedPortal => &[
                Self {
                    placement: VerticalPlacement::Underground,
                    air_pocket_probability: 1.0,
                    mossiness: 0.2,
                    overgrown: false,
                    vines: false,
                    can_be_cold: true,
                    replace_with_blackstone: false,
                    weight: 0.5,
                },
                Self {
                    placement: VerticalPlacement::OnLandSurface,
                    air_pocket_probability: 0.5,
                    mossiness: 0.2,
                    overgrown: false,
                    vines: false,
                    can_be_cold: true,
                    replace_with_blackstone: false,
                    weight: 0.5,
                },
            ],
            StructureKeys::RuinedPortalMountain => &[
                Self {
                    placement: VerticalPlacement::InMountain,
                    air_pocket_probability: 1.0,
                    mossiness: 0.2,
                    overgrown: false,
                    vines: false,
                    can_be_cold: true,
                    replace_with_blackstone: false,
                    weight: 0.5,
                },
                Self {
                    placement: VerticalPlacement::OnLandSurface,
                    air_pocket_probability: 0.5,
                    mossiness: 0.2,
                    overgrown: false,
                    vines: false,
                    can_be_cold: true,
                    replace_with_blackstone: false,
                    weight: 0.5,
                },
            ],
            StructureKeys::RuinedPortalDesert => &[Self {
                placement: VerticalPlacement::PartlyBuried,
                air_pocket_probability: 0.0,
                mossiness: 0.0,
                overgrown: false,
                vines: false,
                can_be_cold: false,
                replace_with_blackstone: false,
                weight: 1.0,
            }],
            StructureKeys::RuinedPortalJungle => &[Self {
                placement: VerticalPlacement::OnLandSurface,
                air_pocket_probability: 0.5,
                mossiness: 0.8,
                overgrown: true,
                vines: true,
                can_be_cold: false,
                replace_with_blackstone: false,
                weight: 1.0,
            }],
            StructureKeys::RuinedPortalSwamp => &[Self {
                placement: VerticalPlacement::OnOceanFloor,
                air_pocket_probability: 0.0,
                mossiness: 0.5,
                overgrown: false,
                vines: true,
                can_be_cold: false,
                replace_with_blackstone: false,
                weight: 1.0,
            }],
            StructureKeys::RuinedPortalOcean => &[Self {
                placement: VerticalPlacement::OnOceanFloor,
                air_pocket_probability: 0.0,
                mossiness: 0.8,
                overgrown: false,
                vines: false,
                can_be_cold: true,
                replace_with_blackstone: false,
                weight: 1.0,
            }],
            StructureKeys::RuinedPortalNether => &[Self {
                placement: VerticalPlacement::InNether,
                air_pocket_probability: 0.5,
                mossiness: 0.0,
                overgrown: false,
                vines: false,
                can_be_cold: false,
                replace_with_blackstone: true,
                weight: 1.0,
            }],
            _ => &[Self {
                placement: VerticalPlacement::OnLandSurface,
                air_pocket_probability: 0.5,
                mossiness: 0.2,
                overgrown: false,
                vines: false,
                can_be_cold: true,
                replace_with_blackstone: false,
                weight: 1.0,
            }],
        }
    }
}

impl RuinedPortalProperties {
    #[must_use]
    pub const fn for_variant(variant: StructureKeys) -> (VerticalPlacement, Self) {
        let setup = &RuinedPortalSetup::for_variant(variant)[0];
        (
            setup.placement,
            Self {
                cold: setup.can_be_cold,
                mossiness: setup.mossiness,
                air_pocket: setup.air_pocket_probability >= 0.5,
                overgrown: setup.overgrown,
                vines: setup.vines,
                replace_with_blackstone: setup.replace_with_blackstone,
            },
        )
    }
}

pub struct RuinedPortalGenerator {
    pub variant: StructureKeys,
}

impl StructureGenerator for RuinedPortalGenerator {
    fn get_structure_position(
        &self,
        mut context: StructureGeneratorContext<'_>,
    ) -> Option<StructurePosition> {
        let setups = RuinedPortalSetup::for_variant(self.variant);
        let chosen_setup = if setups.len() > 1 {
            let total: f32 = setups.iter().map(|s| s.weight).sum();
            let mut pick = context.random.next_f32();
            let mut selected = &setups[0];
            for s in setups {
                pick -= s.weight / total;
                if pick < 0.0 {
                    selected = s;
                    break;
                }
            }
            selected
        } else {
            &setups[0]
        };

        // Vanilla sample(random, setup.airPocketProbability()):
        // limit == 0.0 -> false, limit == 1.0 -> true, otherwise random.nextFloat() < limit.
        let air_pocket = if chosen_setup.air_pocket_probability == 0.0 {
            false
        } else if chosen_setup.air_pocket_probability == 1.0 {
            true
        } else {
            context.random.next_f32() < chosen_setup.air_pocket_probability
        };

        // 5% chance for giant portal
        let pool = if context.random.next_f32() < 0.05 {
            GIANT_PORTALS
        } else {
            PORTALS
        };

        let template_idx = context.random.next_bounded_i32(pool.len() as i32) as usize;
        let template_name = pool[template_idx];
        let template = get_template(template_name)?;

        let rotation_idx = context.random.next_bounded_i32(4) as u8;
        let rotation = Rotation::from_index(rotation_idx);

        let mirror = if context.random.next_f32() < 0.5 {
            Mirror::None
        } else {
            Mirror::FrontBack
        };

        let base_x = start_block_x(context.chunk_x);
        let base_z = start_block_z(context.chunk_z);
        let pivot = Vector3::new(template.size.x / 2, 0, template.size.z / 2);

        let bounding_box = template.get_bounding_box_with_pivot(
            Vector3::new(base_x, 0, base_z),
            rotation,
            pivot,
            mirror,
        );
        let center = bounding_box.center();

        let surface_y = match chosen_setup.placement {
            VerticalPlacement::OnOceanFloor => context
                .height_sampler
                .as_deref_mut()
                .map_or(context.sea_level, |s| {
                    s.estimate_ocean_floor_height(center.x, center.z)
                }) - 1,
            _ => context
                .height_sampler
                .as_deref_mut()
                .map_or(64, |s| s.estimate_height(center.x, center.z)) - 1,
        };

        let y_span = template.size.y as i32;
        let min_y = context.min_y + 15;

        let projected_y = match chosen_setup.placement {
            VerticalPlacement::InNether => {
                if air_pocket {
                    context.random.next_inbetween_i32(32, 100)
                } else if context.random.next_f32() < 0.5 {
                    context.random.next_inbetween_i32(27, 29)
                } else {
                    context.random.next_inbetween_i32(29, 100)
                }
            }
            VerticalPlacement::InMountain => {
                let max_y = surface_y - y_span;
                if 70 < max_y {
                    context.random.next_inbetween_i32(70, max_y)
                } else {
                    max_y
                }
            }
            VerticalPlacement::Underground => {
                let max_y = surface_y - y_span;
                if min_y < max_y {
                    context.random.next_inbetween_i32(min_y, max_y)
                } else {
                    max_y
                }
            }
            VerticalPlacement::PartlyBuried => {
                surface_y - y_span + context.random.next_inbetween_i32(2, 8)
            }
            _ => surface_y,
        };

        let properties = RuinedPortalProperties {
            cold: chosen_setup.can_be_cold,
            mossiness: chosen_setup.mossiness,
            air_pocket,
            overgrown: chosen_setup.overgrown,
            vines: chosen_setup.vines,
            replace_with_blackstone: chosen_setup.replace_with_blackstone,
        };

        let template_position = Vector3::new(base_x, projected_y, base_z);
        let piece = RuinedPortalPiece::new(
            template,
            template_name.to_string(),
            template_position,
            chosen_setup.placement,
            properties,
            rotation,
            mirror,
            pivot,
        );

        let mut collector = StructurePiecesCollector::default();
        collector.add_piece(Box::new(piece));

        // Vanilla returns BlockPos(basePosition.getX(), projectedY, basePosition.getZ())
        // which corresponds to (base_x, projected_y, base_z).
        Some(StructurePosition::new(
            BlockPos::new(base_x, projected_y, base_z),
            collector,
        ))
    }
}

pub struct RuinedPortalPiece {
    pub piece: StructurePiece,
    pub template: Arc<StructureTemplate>,
    pub template_name: String,
    pub place_settings: StructurePlaceSettings,
    pub template_position: Vector3<i32>,
    pub vertical_placement: VerticalPlacement,
    pub properties: RuinedPortalProperties,
}

impl RuinedPortalPiece {
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        template: Arc<StructureTemplate>,
        template_name: String,
        template_position: Vector3<i32>,
        vertical_placement: VerticalPlacement,
        properties: RuinedPortalProperties,
        rotation: Rotation,
        mirror: Mirror,
        pivot: Vector3<i32>,
    ) -> Self {
        let place_settings =
            make_settings(mirror, rotation, vertical_placement, pivot, &properties);
        let bounding_box = template.get_bounding_box(&place_settings, template_position);

        Self {
            piece: StructurePiece::new(StructurePieceType::RuinedPortal, bounding_box, 0),
            template,
            template_name,
            place_settings,
            template_position,
            vertical_placement,
            properties,
        }
    }

    fn place_blocks(&self, chunk: &mut ProtoChunk, chunk_box: &BlockBox) {
        let rotation = self.place_settings.get_rotation();
        let mirror = self.place_settings.get_mirror();
        let pivot = self.place_settings.get_rotation_pivot();

        let mut context_rng = LegacyRand::from_seed(hash_block_pos(
            self.template_position.x,
            self.template_position.y,
            self.template_position.z,
        ) as u64);
        let mut context = ProcessorContext::new(
            self.template_position,
            self.place_settings.get_processors(),
            &mut context_rng,
        );

        for block in &self.template.blocks {
            let palette_entry = &self.template.palette[block.state as usize];

            let mut block_entity_nbt = block.nbt.clone();
            let placed_entry = palette_entry.clone();

            let Some(mut state) = BlockStateResolver::resolve(&placed_entry, rotation, mirror)
            else {
                continue;
            };

            let local_pos =
                StructureTemplate::transform_block_pos(block.pos, mirror, rotation, pivot);
            let mut world_pos = self.template_position + local_pos;

            if !chunk_box.contains_pos(&world_pos) {
                continue;
            }

            let mut processed_state = Some(state);
            let mut capped_idx = 0;
            for processor in self.place_settings.get_processors() {
                let Some(current_state) = processed_state else {
                    break;
                };
                processed_state = processor.process_with_context(
                    chunk,
                    &mut world_pos,
                    current_state,
                    &mut block_entity_nbt,
                    &mut context,
                    &mut capped_idx,
                    &mut context_rng,
                );
            }

            let Some(final_state) = processed_state else {
                continue;
            };

            if chunk.get_block_state(&world_pos).to_block_id() == Block::WATER.id
                && let Some((_, waterlogged)) = placed_entry
                    .properties
                    .iter()
                    .find(|(name, _)| name == "waterlogged")
                && waterlogged == "true"
            {
                if let Some(waterlogged_state) =
                    BlockStateResolver::resolve(&placed_entry, rotation, mirror)
                {
                    state = waterlogged_state;
                } else {
                    state = final_state;
                }
            } else {
                state = final_state;
            }

            chunk.set_block_state(world_pos.x, world_pos.y, world_pos.z, state);

            let final_block = Block::from_id(state.id.to_block_id());
            let block_entity_id =
                crate::generation::structure::template::get_block_entity_id(final_block.name);
            if block_entity_nbt.is_some() || block_entity_id.is_some() {
                let fallback_id = block_entity_id.unwrap_or(final_block.name);
                let mut placed_nbt = pumpkin_nbt::compound::NbtCompound::new();

                placed_nbt.put_string("id", fallback_id.to_string());
                placed_nbt.put_int("x", world_pos.x);
                placed_nbt.put_int("y", world_pos.y);
                placed_nbt.put_int("z", world_pos.z);

                if let Some(template_nbt) = &block_entity_nbt {
                    for (key, value) in &template_nbt.child_tags {
                        if key.as_ref() != "x"
                            && key.as_ref() != "y"
                            && key.as_ref() != "z"
                            && key.as_ref() != "id"
                        {
                            placed_nbt.child_tags.insert(key.clone(), value.clone());
                        }
                    }
                }

                if placed_nbt.get_string("LootTable").is_some()
                    && placed_nbt.get_long("LootTableSeed").is_none()
                {
                    let mut rng =
                        LegacyRand::from_seed(
                            hash_block_pos(world_pos.x, world_pos.y, world_pos.z) as u64,
                        );
                    placed_nbt.put_long("LootTableSeed", rng.next_i64());
                }

                chunk.add_block_entity(placed_nbt);
            }
        }
    }

    fn spread_netherrack(
        &self,
        random: &mut RandomGenerator,
        chunk: &mut ProtoChunk,
        chunk_box: &BlockBox,
    ) {
        let follow_ground_surface = self.vertical_placement == VerticalPlacement::OnLandSurface
            || self.vertical_placement == VerticalPlacement::OnOceanFloor;
        let bb = self.piece.bounding_box;
        let center_x = i32::midpoint(bb.min.x, bb.max.x);
        let center_z = i32::midpoint(bb.min.z, bb.max.z);

        let max_distance = NETHERRACK_PROBABILITY_BY_DISTANCE.len() as i32;
        let x_span = bb.max.x - bb.min.x + 1;
        let z_span = bb.max.z - bb.min.z + 1;
        let average_width = i32::midpoint(x_span, z_span);
        let max_adj = (8 - average_width / 2).max(1);
        let distance_adjustment = random.next_bounded_i32(max_adj);

        let heightmap = self.vertical_placement.get_heightmap_type();

        for x in (center_x - max_distance)..=(center_x + max_distance) {
            for z in (center_z - max_distance)..=(center_z + max_distance) {
                let distance = (x - center_x).abs() + (z - center_z).abs();
                let adjusted_distance = (distance + distance_adjustment).max(0);
                if adjusted_distance < max_distance {
                    let prob = NETHERRACK_PROBABILITY_BY_DISTANCE[adjusted_distance as usize];
                    if random.next_f64() < prob as f64 {
                        let surface_y = chunk.get_top_y(&heightmap, x, z) - 1;
                        let y = if follow_ground_surface {
                            surface_y
                        } else {
                            bb.min.y.min(surface_y)
                        };
                        let pos = Vector3::new(x, y, z);
                        if (y - bb.min.y).abs() <= 3
                            && chunk_box.contains_pos(&pos)
                            && self.can_block_be_replaced_by_netherrack_or_magma(chunk, pos)
                        {
                            self.place_netherrack_or_magma(random, chunk, pos);
                            if self.properties.overgrown {
                                Self::maybe_add_leaves_above(random, chunk, pos);
                            }
                            self.add_netherrack_drip_column(
                                random,
                                chunk,
                                pos + Vector3::new(0, -1, 0),
                                chunk_box,
                            );
                        }
                    }
                }
            }
        }
    }

    fn can_block_be_replaced_by_netherrack_or_magma(
        &self,
        chunk: &ProtoChunk,
        pos: Vector3<i32>,
    ) -> bool {
        let state = chunk.get_block_state(&pos);
        let block_id = state.to_block_id();
        block_id != Block::AIR.id
            && block_id != Block::OBSIDIAN.id
            && block_id != Block::BEDROCK.id
            && (self.vertical_placement == VerticalPlacement::InNether
                || block_id != Block::LAVA.id)
    }

    fn place_netherrack_or_magma(
        &self,
        random: &mut RandomGenerator,
        chunk: &mut ProtoChunk,
        pos: Vector3<i32>,
    ) {
        if !self.properties.cold && random.next_f32() < 0.07 {
            chunk.set_block_state(pos.x, pos.y, pos.z, Block::MAGMA_BLOCK.default_state);
        } else {
            chunk.set_block_state(pos.x, pos.y, pos.z, Block::NETHERRACK.default_state);
        }
    }

    fn add_netherrack_drip_columns_below_portal(
        &self,
        random: &mut RandomGenerator,
        chunk: &mut ProtoChunk,
        chunk_box: &BlockBox,
    ) {
        let bb = self.piece.bounding_box;
        for x in (bb.min.x + 1)..bb.max.x {
            for z in (bb.min.z + 1)..bb.max.z {
                let pos = Vector3::new(x, bb.min.y, z);
                if chunk.get_block_state(&pos).to_block_id() == Block::NETHERRACK.id {
                    self.add_netherrack_drip_column(
                        random,
                        chunk,
                        Vector3::new(x, bb.min.y - 1, z),
                        chunk_box,
                    );
                }
            }
        }
    }

    fn add_netherrack_drip_column(
        &self,
        random: &mut RandomGenerator,
        chunk: &mut ProtoChunk,
        start_pos: Vector3<i32>,
        chunk_box: &BlockBox,
    ) {
        let mut cur = start_pos;
        if chunk_box.contains_pos(&cur) {
            self.place_netherrack_or_magma(random, chunk, cur);
        }
        let mut remaining = 8;
        while remaining > 0 && random.next_f32() < 0.5 {
            cur.y -= 1;
            remaining -= 1;
            if chunk_box.contains_pos(&cur) {
                self.place_netherrack_or_magma(random, chunk, cur);
            }
        }
    }

    fn maybe_add_vines(random: &mut RandomGenerator, chunk: &mut ProtoChunk, pos: Vector3<i32>) {
        let state = chunk.get_block_state(&pos);
        let block_id = state.to_block_id();
        if block_id != Block::AIR.id && block_id != Block::VINE.id {
            let dir_idx = random.next_bounded_i32(4);
            let (dir_offset, vine_face) = match dir_idx {
                0 => (Vector3::new(0, 0, -1), "south"),
                1 => (Vector3::new(0, 0, 1), "north"),
                2 => (Vector3::new(-1, 0, 0), "east"),
                _ => (Vector3::new(1, 0, 0), "west"),
            };
            let neighbor_pos = pos + dir_offset;
            if chunk.get_block_state(&neighbor_pos).to_block_id() == Block::AIR.id {
                let entry = PaletteEntry::with_properties(
                    "minecraft:vine".to_string(),
                    vec![(vine_face.to_string(), "true".to_string())],
                );
                if let Some(vine_state) =
                    BlockStateResolver::resolve(&entry, Rotation::None, Mirror::None)
                {
                    chunk.set_block_state(
                        neighbor_pos.x,
                        neighbor_pos.y,
                        neighbor_pos.z,
                        vine_state,
                    );
                }
            }
        }
    }

    fn maybe_add_leaves_above(
        random: &mut RandomGenerator,
        chunk: &mut ProtoChunk,
        pos: Vector3<i32>,
    ) {
        if random.next_f32() < 0.5
            && chunk.get_block_state(&pos).to_block_id() == Block::NETHERRACK.id
        {
            let above = pos + Vector3::new(0, 1, 0);
            if chunk.get_block_state(&above).to_block_id() == Block::AIR.id {
                let entry = PaletteEntry::with_properties(
                    "minecraft:jungle_leaves".to_string(),
                    vec![("persistent".to_string(), "true".to_string())],
                );
                if let Some(leaves_state) =
                    BlockStateResolver::resolve(&entry, Rotation::None, Mirror::None)
                {
                    chunk.set_block_state(above.x, above.y, above.z, leaves_state);
                }
            }
        }
    }
}

impl StructurePieceBase for RuinedPortalPiece {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn get_structure_piece(&self) -> &StructurePiece {
        &self.piece
    }
    fn get_structure_piece_mut(&mut self) -> &mut StructurePiece {
        &mut self.piece
    }
    fn place(
        &mut self,
        chunk: &mut ProtoChunk,
        _block_registry: &dyn WorldPortalExt,
        random: &mut RandomGenerator,
        _seed: i64,
        chunk_box: &BlockBox,
    ) {
        // Vanilla's postProcess only places blocks from the chunk that contains the
        // structure's center (it uses a WorldGenLevel for cross-chunk access). In
        // Pumpkin, ProtoChunk clips writes to its own 16×16 column, so we instead
        // call place for *every* chunk that intersects (generate_in_chunk in mod.rs
        // already handles that) and let chunk_box act as the per-chunk clip region.
        // We must NOT enlarge chunk_box or the coordinate wrap-around in set_block_state
        // will silently corrupt other positions.

        self.place_blocks(chunk, chunk_box);
        self.spread_netherrack(random, chunk, chunk_box);
        self.add_netherrack_drip_columns_below_portal(random, chunk, chunk_box);

        if self.properties.vines || self.properties.overgrown {
            for x in self.piece.bounding_box.min.x..=self.piece.bounding_box.max.x {
                for y in self.piece.bounding_box.min.y..=self.piece.bounding_box.max.y {
                    for z in self.piece.bounding_box.min.z..=self.piece.bounding_box.max.z {
                        let pos = Vector3::new(x, y, z);
                        if chunk_box.contains_pos(&pos) {
                            if self.properties.vines {
                                Self::maybe_add_vines(random, chunk, pos);
                            }
                            if self.properties.overgrown {
                                Self::maybe_add_leaves_above(random, chunk, pos);
                            }
                        }
                    }
                }
            }
        }
    }
}

fn make_settings(
    mirror: Mirror,
    rotation: Rotation,
    vertical_placement: VerticalPlacement,
    pivot: Vector3<i32>,
    properties: &RuinedPortalProperties,
) -> StructurePlaceSettings {
    let ignore_processor = if properties.air_pocket {
        StructureProcessor::BlockIgnore(vec![IgnoredBlock {
            block_id: Block::STRUCTURE_BLOCK.id,
            properties: None,
        }])
    } else {
        StructureProcessor::BlockIgnore(vec![
            IgnoredBlock {
                block_id: Block::STRUCTURE_BLOCK.id,
                properties: None,
            },
            IgnoredBlock {
                block_id: Block::AIR.id,
                properties: None,
            },
        ])
    };

    let mut rules = Vec::new();
    rules.push(ProcessorRule {
        position_predicate: PosRuleTest::AlwaysTrue,
        input_predicate: RuleTest::RandomBlockMatch {
            block: Block::GOLD_BLOCK.id,
            probability: 0.3,
        },
        location_predicate: RuleTest::AlwaysTrue,
        output_state: Block::AIR.default_state,
        block_entity_modifier: None,
    });

    let lava_rule = match vertical_placement {
        VerticalPlacement::OnOceanFloor => ProcessorRule {
            position_predicate: PosRuleTest::AlwaysTrue,
            input_predicate: RuleTest::BlockMatch(Block::LAVA.id),
            location_predicate: RuleTest::AlwaysTrue,
            output_state: Block::MAGMA_BLOCK.default_state,
            block_entity_modifier: None,
        },
        _ => {
            if properties.cold {
                ProcessorRule {
                    position_predicate: PosRuleTest::AlwaysTrue,
                    input_predicate: RuleTest::BlockMatch(Block::LAVA.id),
                    location_predicate: RuleTest::AlwaysTrue,
                    output_state: Block::NETHERRACK.default_state,
                    block_entity_modifier: None,
                }
            } else {
                ProcessorRule {
                    position_predicate: PosRuleTest::AlwaysTrue,
                    input_predicate: RuleTest::RandomBlockMatch {
                        block: Block::LAVA.id,
                        probability: 0.2,
                    },
                    location_predicate: RuleTest::AlwaysTrue,
                    output_state: Block::MAGMA_BLOCK.default_state,
                    block_entity_modifier: None,
                }
            }
        }
    };
    rules.push(lava_rule);

    if !properties.cold {
        rules.push(ProcessorRule {
            position_predicate: PosRuleTest::AlwaysTrue,
            input_predicate: RuleTest::RandomBlockMatch {
                block: Block::NETHERRACK.id,
                probability: 0.07,
            },
            location_predicate: RuleTest::AlwaysTrue,
            output_state: Block::MAGMA_BLOCK.default_state,
            block_entity_modifier: None,
        });
    }

    let mut settings = StructurePlaceSettings::new()
        .set_rotation(rotation)
        .set_mirror(mirror)
        .set_rotation_pivot(pivot)
        .add_processor(ignore_processor)
        .add_processor(StructureProcessor::Rule(rules))
        .add_processor(StructureProcessor::BlockAge {
            mossiness: properties.mossiness,
        })
        .add_processor(StructureProcessor::ProtectedBlocks(
            "#minecraft:features_cannot_replace".to_string(),
        ))
        .add_processor(StructureProcessor::LavaSubmergedBlock);

    if properties.replace_with_blackstone {
        settings = settings.add_processor(StructureProcessor::BlackstoneReplace);
    }

    settings
}
