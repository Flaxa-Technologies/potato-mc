use std::collections::HashMap;
use std::sync::Arc;

use pumpkin_data::{
    Block, BlockState, BlockStateId,
    damage::DamageType,
    entity::EntityType,
    fluid::Fluid,
    particle::Particle,
    sound::Sound,
    tag::{Tag, Taggable},
};
use pumpkin_util::math::{boundingbox::BoundingBox, position::BlockPos, vector3::Vector3};
use pumpkin_world::chunk::ChunkData;
use rustc_hash::FxHashMap;

use pumpkin_data::item_stack::ItemStack;

use crate::{
    block::ExplodeArgs,
    entity::{Entity, EntityBase},
    world::loot::LootContextParameters,
};

use super::{BlockFlags, World};

/// Defines the type of explosion interaction with the world.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExplosionInteraction {
    None,
    Block,
    Mob,
    Tnt,
    Trigger,
}

/// Defines how an explosion interacts with blocks in the world.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockInteraction {
    /// Keeps blocks intact (no block damage, no drops).
    Keep,
    /// Destroys blocks and drops 100% of items without decay.
    Destroy,
    /// Destroys blocks and applies loot decay based on explosion radius.
    DestroyWithDecay,
    /// Triggers block effects without destroying them.
    TriggerBlock,
}

impl BlockInteraction {
    #[must_use]
    pub const fn should_affect_blocklike_entities(self) -> bool {
        matches!(self, Self::Destroy | Self::DestroyWithDecay)
    }
}

/// Defines how damage and block destruction are calculated for an explosion.
pub trait ExplosionDamageCalculator: Send + Sync {
    /// Returns the block's explosion resistance. If None, the block is treated as air/empty.
    fn get_block_explosion_resistance(
        &self,
        _explosion: &Explosion,
        _world: &World,
        _pos: &BlockPos,
        block: &Block,
        fluid: &pumpkin_data::fluid::FluidState,
    ) -> Option<f32> {
        if block.default_state.is_air() && fluid.is_empty {
            None
        } else {
            Some(fluid.blast_resistance.max(block.blast_resistance))
        }
    }

    /// Returns whether this block should be destroyed / affected by the explosion.
    fn should_block_explode(
        &self,
        _explosion: &Explosion,
        _world: &World,
        _pos: &BlockPos,
        _block: &Block,
        _power: f32,
    ) -> bool {
        true
    }

    /// Returns whether the entity should take damage from the explosion.
    fn should_damage_entity(&self, _explosion: &Explosion, _entity: &dyn EntityBase) -> bool {
        true
    }

    /// Returns knockback multiplier for the given entity (default 1.0).
    fn get_knockback_multiplier(&self, _entity: &dyn EntityBase) -> f32 {
        1.0
    }

    /// Calculates the damage amount to deal to the entity given the exposure.
    fn get_entity_damage_amount(
        &self,
        explosion: &Explosion,
        entity: &dyn EntityBase,
        exposure: f32,
    ) -> f32 {
        let radius = explosion.power as f64 * 2.0;
        let distance = (entity
            .get_entity()
            .pos
            .load()
            .squared_distance_to_vec(&explosion.pos))
        .sqrt()
            / radius;
        let damage_multiplier = (1.0 - distance) * exposure as f64;
        (f64::midpoint(damage_multiplier * damage_multiplier, damage_multiplier)
            * 7.0
            * radius
            + 1.0) as f32
    }
}

/// Default explosion damage calculator implementing vanilla standard explosion rules.
pub struct DefaultExplosionDamageCalculator;

impl ExplosionDamageCalculator for DefaultExplosionDamageCalculator {}

/// A configurable explosion damage calculator (e.g. for wind charges, mace wind bursts).
pub struct SimpleExplosionDamageCalculator {
    pub damages_entities: bool,
    pub damages_blocks: bool,
    pub knockback_multiplier: Option<f32>,
    pub immune_blocks: Option<&'static Tag>,
}

impl SimpleExplosionDamageCalculator {
    #[must_use]
    pub const fn new(
        damages_entities: bool,
        damages_blocks: bool,
        knockback_multiplier: Option<f32>,
        immune_blocks: Option<&'static Tag>,
    ) -> Self {
        Self {
            damages_entities,
            damages_blocks,
            knockback_multiplier,
            immune_blocks,
        }
    }
}

impl ExplosionDamageCalculator for SimpleExplosionDamageCalculator {
    fn get_block_explosion_resistance(
        &self,
        _explosion: &Explosion,
        _world: &World,
        _pos: &BlockPos,
        block: &Block,
        fluid: &pumpkin_data::fluid::FluidState,
    ) -> Option<f32> {
        if let Some(immune_tag) = self.immune_blocks
            && block.has_tag(immune_tag)
        {
            return Some(3_600_000.0);
        }
        if block.default_state.is_air() && fluid.is_empty {
            None
        } else {
            Some(fluid.blast_resistance.max(block.blast_resistance))
        }
    }

    fn should_block_explode(
        &self,
        explosion: &Explosion,
        _world: &World,
        _pos: &BlockPos,
        block: &Block,
        _power: f32,
    ) -> bool {
        if let Some(immune_tag) = self.immune_blocks
            && block.has_tag(immune_tag)
        {
            return false;
        }
        if explosion.block_interaction == BlockInteraction::TriggerBlock {
            return true;
        }
        if !self.damages_blocks {
            return false;
        }
        true
    }

    fn should_damage_entity(&self, _explosion: &Explosion, _entity: &dyn EntityBase) -> bool {
        self.damages_entities
    }

    fn get_knockback_multiplier(&self, _entity: &dyn EntityBase) -> f32 {
        self.knockback_multiplier.unwrap_or(1.0)
    }
}

pub struct Explosion {
    power: f32,
    pos: Vector3<f64>,
    block_interaction: BlockInteraction,
    damage_calculator: Option<Arc<dyn ExplosionDamageCalculator>>,
    preserve_rails: bool,
    sound: Option<Sound>,
    particle: Option<Particle>,
    is_wind_charge: bool,
}

impl Explosion {
    #[must_use]
    pub const fn new(power: f32, pos: Vector3<f64>, block_interaction: BlockInteraction) -> Self {
        Self {
            power,
            pos,
            block_interaction,
            damage_calculator: None,
            preserve_rails: false,
            sound: None,
            particle: None,
            is_wind_charge: false,
        }
    }

    #[must_use]
    pub fn with_damage_calculator(
        mut self,
        calculator: Arc<dyn ExplosionDamageCalculator>,
    ) -> Self {
        self.damage_calculator = Some(calculator);
        self
    }

    #[must_use]
    pub const fn with_sound(mut self, sound: Sound) -> Self {
        self.sound = Some(sound);
        self
    }

    #[must_use]
    pub const fn with_particle(mut self, particle: Particle) -> Self {
        self.particle = Some(particle);
        self
    }

    #[must_use]
    pub const fn with_wind_charge(mut self, is_wind_charge: bool) -> Self {
        self.is_wind_charge = is_wind_charge;
        self
    }

    #[must_use]
    pub const fn sound(&self) -> Option<Sound> {
        self.sound
    }

    #[must_use]
    pub const fn particle(&self) -> Option<Particle> {
        self.particle
    }

    #[must_use]
    pub const fn is_wind_charge(&self) -> bool {
        self.is_wind_charge
    }

    #[must_use]
    pub const fn preserving_rails(mut self) -> Self {
        self.preserve_rails = true;
        self
    }

    fn protects_rail(&self, world: &World, pos: &BlockPos, block: &Block) -> bool {
        self.preserve_rails && (Self::is_rail(block) || Self::is_rail(world.get_block(&pos.up())))
    }

    fn is_rail(block: &Block) -> bool {
        block.id == Block::RAIL.id
            || block.id == Block::POWERED_RAIL.id
            || block.id == Block::DETECTOR_RAIL.id
            || block.id == Block::ACTIVATOR_RAIL.id
    }

    #[allow(clippy::too_many_lines)]
    fn get_blocks_to_destroy(
        &self,
        world: &World,
    ) -> FxHashMap<BlockPos, (&'static Block, &'static BlockState)> {
        let mut map = FxHashMap::default();

        let mut chunk_cache: FxHashMap<
            pumpkin_util::math::vector2::Vector2<i32>,
            Option<Arc<ChunkData>>,
        > = FxHashMap::default();

        let default_calc = DefaultExplosionDamageCalculator;
        let calc: &dyn ExplosionDamageCalculator = match &self.damage_calculator {
            Some(c) => c.as_ref(),
            None => &default_calc,
        };

        for x in 0..16 {
            for y in 0..16 {
                for z in 0..16 {
                    if x > 0 && x < 15 && y > 0 && y < 15 && z > 0 && z < 15 {
                        continue;
                    }

                    let mut dir_x = f64::from(x) / 7.5 - 1.0;
                    let mut dir_y = f64::from(y) / 7.5 - 1.0;
                    let mut dir_z = f64::from(z) / 7.5 - 1.0;

                    let length = (dir_x * dir_x + dir_y * dir_y + dir_z * dir_z).sqrt();
                    dir_x /= length;
                    dir_y /= length;
                    dir_z /= length;

                    let mut pos_x = self.pos.x;
                    let mut pos_y = self.pos.y;
                    let mut pos_z = self.pos.z;

                    let random_val = rand::random::<f32>();
                    let mut h = self.power * random_val.mul_add(0.6, 0.7);

                    while h > 0.0 {
                        let block_pos = BlockPos::floored(pos_x, pos_y, pos_z);

                        if !world.is_in_build_limit(block_pos) {
                            break;
                        }

                        let (chunk_pos, relative) = block_pos.chunk_and_chunk_relative_position();

                        let chunk_opt = chunk_cache.entry(chunk_pos).or_insert_with(|| {
                            world
                                .level
                                .read_chunk_sync(&chunk_pos, std::clone::Clone::clone)
                        });

                        let state_id = if let Some(chunk) = chunk_opt {
                            chunk
                                .section
                                .get_block_absolute_y(
                                    relative.x as usize,
                                    block_pos.0.y,
                                    relative.z as usize,
                                )
                                .unwrap_or(Block::AIR.default_state.id)
                        } else {
                            Block::AIR.default_state.id
                        };

                        let (block, state) = BlockState::from_id_with_block(state_id);

                        let (_fluid, fluid_state) = Fluid::from_state_id(state_id).map_or_else(
                            || {
                                if block.is_waterlogged(state_id) {
                                    (&Fluid::FLOWING_WATER, &Fluid::FLOWING_WATER.states[0])
                                } else {
                                    (&Fluid::EMPTY, &Fluid::EMPTY.states[0])
                                }
                            },
                            |raw_fluid| {
                                let f = raw_fluid.to_flowing();
                                (f, &f.states[0])
                            },
                        );

                        if !state.is_air() || !fluid_state.is_empty {
                            let protects_rail = self.protects_rail(world, &block_pos, block);
                            let resistance = if protects_rail {
                                Some(0.0)
                            } else {
                                calc.get_block_explosion_resistance(
                                    self,
                                    world,
                                    &block_pos,
                                    block,
                                    fluid_state,
                                )
                            };

                            if let Some(resistance) = resistance {
                                h -= (resistance + 0.3) * 0.3;
                            }

                            if h > 0.0
                                && !protects_rail
                                && calc.should_block_explode(self, world, &block_pos, block, h)
                            {
                                map.insert(block_pos, (block, state));
                            }
                        }

                        pos_x += dir_x * 0.3;
                        pos_y += dir_y * 0.3;
                        pos_z += dir_z * 0.3;
                        h -= 0.225_000_01;
                    }
                }
            }
        }
        map
    }

    fn damage_entities(&self, world: &Arc<World>) -> HashMap<i32, Vector3<f64>> {
        let mut player_knockbacks: HashMap<i32, Vector3<f64>> = HashMap::new();
        // Explosion is too small
        if self.power < 1.0e-5 {
            return player_knockbacks;
        }

        let radius = self.power as f64 * 2.0;
        let min_x = (self.pos.x - radius - 1.0).floor() as i32;
        let max_x = (self.pos.x + radius + 1.0).floor() as i32;
        let min_y = (self.pos.y - radius - 1.0).floor() as i32;
        let max_y = (self.pos.y + radius + 1.0).floor() as i32;
        let min_z = (self.pos.z - radius - 1.0).floor() as i32;
        let max_z = (self.pos.z + radius + 1.0).floor() as i32;

        let search_box = BoundingBox::new(
            Vector3::new(min_x as f64, min_y as f64, min_z as f64),
            Vector3::new(max_x as f64, max_y as f64, max_z as f64),
        );

        let entities = world.get_all_at_box(&search_box);

        let default_calc = DefaultExplosionDamageCalculator;
        let calc: &dyn ExplosionDamageCalculator = match &self.damage_calculator {
            Some(c) => c.as_ref(),
            None => &default_calc,
        };

        for entity_base in entities {
            if entity_base.is_immune_to_explosion() {
                continue;
            }

            // Skip spectators (no damage, no knockback)
            if entity_base.is_spectator() {
                continue;
            }

            let entity = entity_base.get_entity();

            // Vanilla parity: Item and ArmorStand entities ignore explosions unless they affect blocks
            if (entity.entity_type == &EntityType::ITEM
                || entity.entity_type == &EntityType::ARMOR_STAND)
                && !self.block_interaction.should_affect_blocklike_entities()
            {
                continue;
            }

            let distance = (entity.pos.load().squared_distance_to_vec(&self.pos)).sqrt() / radius;
            if distance > 1.0 {
                continue;
            }

            let should_damage = calc.should_damage_entity(self, entity_base.as_ref());
            let knockback_multiplier = calc.get_knockback_multiplier(entity_base.as_ref()) as f64;

            let mut exposure = if !should_damage && knockback_multiplier == 0.0 {
                0.0
            } else {
                Self::calculate_exposure(&self.pos, entity, world) as f64
            };

            if self.is_wind_charge && exposure == 0.0 && distance <= 1.0 {
                let check_pos = entity.get_eye_pos();
                if world
                    .raycast(check_pos, self.pos, |pos, world_ref| {
                        let state = world_ref.get_block_state(pos);
                        !state.is_air() && !state.collision_shapes.is_empty()
                    })
                    .is_none()
                {
                    exposure = 1.0;
                }
            }

            if exposure == 0.0 {
                continue;
            }

            if should_damage {
                let damage =
                    calc.get_entity_damage_amount(self, entity_base.as_ref(), exposure as f32);
                entity_base.damage_with_context(
                    entity_base.as_ref(),
                    damage,
                    DamageType::EXPLOSION,
                    Some(self.pos),
                    None,
                    None,
                );
            }

            // Calculate and apply knockback
            let dir_pos = if entity.entity_type == &EntityType::TNT {
                entity.pos.load()
            } else {
                entity.get_eye_pos()
            };
            let diff = dir_pos - self.pos;
            let dist_len = diff.length();
            let horizontal_dist_sq = diff.x.mul_add(diff.x, diff.z * diff.z);
            let (direction, eff_distance) = if horizontal_dist_sq < 1.0e-4 {
                (Vector3::new(0.0, 1.0, 0.0), 0.0)
            } else if dist_len > 1.0e-5 {
                (diff / dist_len, distance)
            } else {
                (Vector3::new(0.0, 1.0, 0.0), 0.0)
            };
            let knockback_resistance = entity_base
                .get_living_entity()
                .map_or(0.0, |l| {
                    l.get_attribute_value(&pumpkin_data::attributes::Attributes::EXPLOSION_KNOCKBACK_RESISTANCE)
                });

            let knockback_power =
                (1.0 - eff_distance) * exposure * knockback_multiplier * (1.0 - knockback_resistance);
            let knockback = direction * knockback_power;
            if let Some(player) = world.get_player_by_id(entity.entity_id) {
                let is_flying_creative = player.gamemode.load() == pumpkin_util::GameMode::Creative
                    && player
                        .abilities
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .flying;
                if !player.is_spectator() && !is_flying_creative {
                    // Update server-side velocity WITHOUT broadcasting a CEntityVelocity
                    // packet. The CExplosion packet already carries this impulse to the
                    // client; sending CEntityVelocity here too would apply it twice.
                    let cur = player.living_entity.entity.velocity.load();
                    player.living_entity.entity.velocity.store(cur + knockback);
                    player_knockbacks.insert(entity.entity_id, knockback);
                }
                if self.is_wind_charge {
                    player
                        .living_entity
                        .set_ignore_fall_damage_from_current_impulse(true, player.position());
                }
            } else {
                entity.add_velocity(knockback);
            }
        }
        player_knockbacks
    }

    fn calculate_exposure(
        explosion_pos: &Vector3<f64>,
        entity: &Entity,
        world: &Arc<World>,
    ) -> f32 {
        let bbox = entity.bounding_box.load();

        let dx = (bbox.max.x - bbox.min.x).max(0.0);
        let dy = (bbox.max.y - bbox.min.y).max(0.0);
        let dz = (bbox.max.z - bbox.min.z).max(0.0);

        let step_x = 1.0 / (dx * 2.0 + 1.0);
        let step_y = 1.0 / (dy * 2.0 + 1.0);
        let step_z = 1.0 / (dz * 2.0 + 1.0);

        if step_x < 0.0 || step_y < 0.0 || step_z < 0.0 {
            return 0.0;
        }

        let offset_x = (1.0 - (1.0 / step_x).floor() * step_x) / 2.0;
        let offset_z = (1.0 - (1.0 / step_z).floor() * step_z) / 2.0;

        let mut visible_points = 0;
        let mut total_points = 0;

        let mut k = 0.0;
        while k <= 1.0001 {
            let mut l = 0.0;
            while l <= 1.0001 {
                let mut m = 0.0;
                while m <= 1.0001 {
                    let n = bbox.min.x + dx * k;
                    let o = (bbox.min.y + dy * l).max(bbox.min.y + 0.01);
                    let p = bbox.min.z + dz * m;

                    let vec3d = Vector3::new(n + offset_x, o, p + offset_z);

                    if world
                        .raycast(vec3d, *explosion_pos, |pos, world_ref| {
                            let state = world_ref.get_block_state(pos);
                            !state.is_air() && !state.collision_shapes.is_empty()
                        })
                        .is_none()
                    {
                        visible_points += 1;
                    }

                    total_points += 1;
                    m += step_z;
                }
                l += step_y;
            }
            k += step_x;
        }

        if total_points == 0 {
            return 0.0;
        }

        visible_points as f32 / total_points as f32
    }

    /// Returns (removed block count, per-player knockback vectors)
    pub fn explode(&self, world: &Arc<World>) -> (u32, HashMap<i32, Vector3<f64>>) {
        match self.block_interaction {
            BlockInteraction::Keep => {
                let player_knockbacks = self.damage_entities(world);
                (0, player_knockbacks)
            }
            BlockInteraction::TriggerBlock => {
                let player_knockbacks = self.damage_entities(world);
                let blocks = self.get_blocks_to_destroy(world);
                for (pos, (block, _state)) in &blocks {
                    let pumpkin_block = world.block_registry.get_pumpkin_block(block.id);
                    if let Some(pumpkin_block) = pumpkin_block {
                        pumpkin_block.explode(ExplodeArgs {
                            world,
                            block,
                            position: pos,
                        });
                    }
                }
                (0, player_knockbacks)
            }
            BlockInteraction::Destroy | BlockInteraction::DestroyWithDecay => {
                let center_pos = BlockPos::floored(self.pos.x, self.pos.y, self.pos.z);
                let mut event =
                    crate::plugin::api::events::block::block_explode::BlockExplodeEvent::new(
                        center_pos,
                        if self.power > 0.0 {
                            1.0 / self.power
                        } else {
                            1.0
                        },
                    );
                if let Some(server) = world.server.upgrade() {
                    server.plugin_manager.fire_blocking(&server, &mut event);
                }
                if event.cancelled {
                    return (0, HashMap::new());
                }

                // 1. Calculate affected blocks
                let blocks = self.get_blocks_to_destroy(world);
                let decay_drops = self.block_interaction == BlockInteraction::DestroyWithDecay;
                let explosion_radius = decay_drops.then_some(self.power);

                let mut all_drops: Vec<(BlockPos, ItemStack)> = Vec::new();

                // 2. Destroy affected blocks first so they do not block exposure raycasts
                for (pos, (block, state)) in &blocks {
                    world.set_block_state(pos, BlockStateId::AIR, BlockFlags::NOTIFY_ALL);
                    world.close_container_screens_at(pos);

                    let pumpkin_block = world.block_registry.get_pumpkin_block(block.id);

                    if pumpkin_block.is_none_or(|s| s.should_drop_items_on_explosion()) {
                        let is_raining = world.is_raining();
                        let is_thundering = world.is_thundering();
                        let params = LootContextParameters {
                            block_state: Some(state),
                            explosion_radius,
                            position: Some(pumpkin_util::math::vector3::Vector3::new(
                                pos.0.x as f64,
                                pos.0.y as f64,
                                pos.0.z as f64,
                            )),
                            world_time: world.level_info.load().day_time as u64,
                            is_raining: Some(is_raining),
                            is_thundering: Some(is_thundering),
                            ..Default::default()
                        };
                        let key = format!("minecraft:blocks/{}", block.name);
                        if let Some(loot_table) = pumpkin_data::loot_table::get_loot_table(&key) {
                            let seed: i64 = rand::random();
                            let mut items = crate::world::loot::generate_loot_with_context(
                                loot_table, seed, &params,
                            );
                            if block.has_tag(&pumpkin_data::tag::Block::MINECRAFT_LEAVES) {
                                items.retain(|stack| {
                                    stack.item != &pumpkin_data::item::Item::APPLE
                                        && stack.item != &pumpkin_data::item::Item::STICK
                                });
                            }
                            if !items.is_empty() {
                                let mut event = crate::plugin::block::block_drop_item::BlockDropItemEvent {
                                    block_pos: *pos,
                                    world: world.clone(),
                                    player: None,
                                    items,
                                    cancelled: false,
                                };
                                if let Some(server) = world.server.upgrade() {
                                    server.plugin_manager.fire_blocking(&server, &mut event);
                                }
                                if !event.cancelled {
                                    for stack in event.items {
                                        all_drops.push((*pos, stack));
                                    }
                                }
                            }
                        }
                    }
                    if let Some(pumpkin_block) = pumpkin_block {
                        pumpkin_block.explode(ExplodeArgs {
                            world,
                            block,
                            position: pos,
                        });
                    }
                }

                // 3. Damage entities NOW:
                //    - Blocks destroyed by this explosion are now AIR, so line of sight is clear.
                //    - Existing ItemEntities from prior explosions (e.g. TNT #1) within the blast radius
                //      take explosion damage and are removed if lethal.
                //    - Mobs and players take correct damage.
                let player_knockbacks = self.damage_entities(world);

                // 4. Group & spawn the newly generated drops from THIS explosion:
                //    - Spawned AFTER damage_entities, so this explosion's own drops are NOT destroyed.
                //    - Subsequent explosions (e.g. chained TNT #3) will destroy them.
                let mut aggregated: FxHashMap<u16, (ItemStack, u32)> = FxHashMap::default();
                for (_, item) in all_drops {
                    let id = item.item.id;
                    if let Some((existing, total)) = aggregated.get_mut(&id) {
                        if existing.are_items_and_components_equal(&item) {
                            *total += u32::from(item.item_count);
                            continue;
                        }
                    }
                    let count = u32::from(item.item_count);
                    aggregated.entry(id).or_insert((item, 0)).1 += count;
                }

                let explosion_block_pos = BlockPos::new(
                    self.pos.x.floor() as i32,
                    self.pos.y.floor() as i32,
                    self.pos.z.floor() as i32,
                );
                for (_, (template, total_count)) in aggregated {
                    let max_size = u32::from(template.get_max_stack_size());
                    let mut remaining = total_count;
                    while remaining > 0 {
                        let batch = remaining.min(max_size);
                        remaining -= batch;
                        let mut stack = template.clone();
                        stack.item_count = batch as u8;
                        world.drop_stack(&explosion_block_pos, stack);
                    }
                }

                (blocks.len() as u32, player_knockbacks)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Explosion;
    use pumpkin_data::Block;

    #[test]
    fn tnt_minecart_rail_protection_covers_every_rail_type() {
        for rail in [
            &Block::RAIL,
            &Block::POWERED_RAIL,
            &Block::DETECTOR_RAIL,
            &Block::ACTIVATOR_RAIL,
        ] {
            assert!(Explosion::is_rail(rail));
        }
        assert!(!Explosion::is_rail(&Block::STONE));
    }

    #[test]
    fn block_interaction_affecting_blocklike_entities() {
        use super::BlockInteraction;
        assert!(BlockInteraction::Destroy.should_affect_blocklike_entities());
        assert!(BlockInteraction::DestroyWithDecay.should_affect_blocklike_entities());
        assert!(!BlockInteraction::Keep.should_affect_blocklike_entities());
        assert!(!BlockInteraction::TriggerBlock.should_affect_blocklike_entities());
    }
}
