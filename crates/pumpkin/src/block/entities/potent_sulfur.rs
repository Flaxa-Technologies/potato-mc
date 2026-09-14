use std::sync::Arc;
use std::sync::atomic::{AtomicI32, Ordering};

use pumpkin_data::block_properties::{PotentSulfurProperties, PotentSulfurState};
use pumpkin_data::effect::StatusEffect;
use pumpkin_data::particle::Particle;
use pumpkin_data::potion::Effect;
use pumpkin_data::sound::{Sound, SoundCategory};
use pumpkin_data::{Block, BlockId};
use pumpkin_nbt::compound::NbtCompound;
use pumpkin_util::GameMode;
use pumpkin_util::math::boundingbox::BoundingBox;
use pumpkin_util::math::position::BlockPos;
use pumpkin_util::math::vector3::Vector3;
use pumpkin_util::random::xoroshiro128::Xoroshiro;
use pumpkin_util::random::RandomImpl;
use pumpkin_world::world::BlockFlags;

use super::BlockEntity;
use crate::world::World;

pub struct PotentSulfurBlockEntity {
    pub position: BlockPos,
    pub waiting_countdown: AtomicI32,
}

impl BlockEntity for PotentSulfurBlockEntity {
    fn resource_location(&self) -> &'static str {
        Self::ID
    }

    fn get_position(&self) -> BlockPos {
        self.position
    }

    fn from_nbt(nbt: &pumpkin_nbt::compound::NbtCompound, position: BlockPos) -> Self
    where
        Self: Sized,
    {
        let countdown = nbt.get_int("countdown").unwrap_or(-1);
        Self {
            position,
            waiting_countdown: AtomicI32::new(countdown),
        }
    }

    fn write_nbt(&self, nbt: &mut NbtCompound) {
        let countdown = self.waiting_countdown.load(Ordering::Relaxed);
        if countdown != -1 {
            nbt.put_int("countdown", countdown);
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn tick(&self, world: &Arc<World>) {
        let state = world.get_block_state(&self.position);
        if state.id.to_block_id() != BlockId::POTENT_SULFUR {
            return;
        }

        let props = PotentSulfurProperties::from_state_id(state.id);
        let sulfur_state = props.r#potent_sulfur_state;

        if sulfur_state == PotentSulfurState::Dry {
            return;
        }

        let game_time = world.level_time.lock().unwrap_or_else(std::sync::PoisonError::into_inner).world_age;

        // Particle ticker: yellow sulfur bubbles rising through the water column
        if let Some(source_pos) = find_noxious_gas_source_block(world, self.position) {
            let water_height = (source_pos.0.y - self.position.0.y).max(1) as f64;
            let rx = (rand::random::<f32>() * 0.8 + 0.1) as f64;
            let rz = (rand::random::<f32>() * 0.8 + 0.1) as f64;
            let ry = 1.0 + rand::random::<f64>() * (water_height - 0.2).max(0.1);
            let bubble_pos = Vector3::new(
                self.position.0.x as f64 + rx,
                self.position.0.y as f64 + ry,
                self.position.0.z as f64 + rz,
            );
            world.spawn_particle(
                bubble_pos,
                Vector3::new(0.04, 0.08, 0.04),
                0.01,
                2,
                Particle::SulfurBubbles,
            );

            // Noxious gas cloud at water surface every 20 ticks
            if game_time % 20 == 0 {
                let surface_pos = Vector3::new(
                    source_pos.0.x as f64 + 0.5,
                    source_pos.0.y as f64 + 0.2,
                    source_pos.0.z as f64 + 0.5,
                );
                world.spawn_particle(
                    surface_pos,
                    Vector3::new(0.2, 0.1, 0.2),
                    0.02,
                    2,
                    Particle::NoxiousGasCloud,
                );
            }

            // Geyser eruption particles during Erupting or Continuous
            if (sulfur_state == PotentSulfurState::Erupting || sulfur_state == PotentSulfurState::Continuous)
                && game_time % 3 == 0
            {
                let geyser_pos = Vector3::new(
                    source_pos.0.x as f64 + 0.5,
                    source_pos.0.y as f64 + 0.1,
                    source_pos.0.z as f64 + 0.5,
                );
                world.spawn_particle(
                    geyser_pos,
                    Vector3::new(0.1, 0.4, 0.1),
                    0.1,
                    5,
                    Particle::Geyser,
                );
            }
        }

        // 1. Nausea effect ticker (every 10 ticks, for Wet and Dormant states)
        if (sulfur_state == PotentSulfurState::Wet || sulfur_state == PotentSulfurState::Dormant)
            && game_time % 10 == 0
        {
            if let Some(source_pos) = find_noxious_gas_source_block(world, self.position) {
                let aabb = BoundingBox::from_block(&source_pos).expand(2.5, 0.0, 2.5);
                let effect = Effect {
                    effect_type: &StatusEffect::NAUSEA,
                    duration: 80, // 4 seconds
                    amplifier: 0,
                    ambient: true,
                    show_particles: true,
                    show_icon: true,
                    blend: true,
                };

                let source_center = Vector3::new(
                    source_pos.0.x as f64 + 0.5,
                    source_pos.0.y as f64 + 0.5,
                    source_pos.0.z as f64 + 0.5,
                );

                for player in world.get_players_at_box(&aabb) {
                    if player.living_entity.entity.pos.load().squared_distance_to(source_center.x, source_center.y, source_center.z) <= 9.0 {
                        player.add_effect(effect.clone());
                    }
                }

                for entity in world.get_entities_at_box(&aabb) {
                    let base = entity.get_entity();
                    if base.pos.load().squared_distance_to(source_center.x, source_center.y, source_center.z) <= 9.0 {
                        if let Some(living) = entity.get_living_entity() {
                            living.add_effect(effect.clone());
                        }
                    }
                }
            }
        }

        // 2. Waiting countdown ticker (every 20 ticks, for Dormant and Erupting states)
        if (sulfur_state == PotentSulfurState::Dormant || sulfur_state == PotentSulfurState::Erupting)
            && game_time % 20 == 0
        {
            if let Some(source_pos) = find_noxious_gas_source_block(world, self.position) {
                let water_blocks = (source_pos.0.y - self.position.0.y - 1).max(1);
                let mut countdown = self.waiting_countdown.load(Ordering::Relaxed);

                if countdown <= 0 {
                    // Seed from position hash matching vanilla level.getSeed() ^ -904011478L
                    let splitter = Xoroshiro::from_seed(world.level.seed.0 ^ ((-904011478i64) as u64)).next_splitter();
                    let mut rng = splitter.split_pos(self.position.0.x, self.position.0.y, self.position.0.z);

                    if sulfur_state == PotentSulfurState::Dormant {
                        countdown = 10 * (water_blocks - 1) + rng.next_bounded_i32(16) + 15;
                    } else {
                        let _ = rng.next_i32();
                        countdown = (water_blocks - 1) + rng.next_bounded_i32(2) + 1;
                    }
                    self.waiting_countdown.store(countdown, Ordering::Relaxed);
                }

                if countdown > 0 {
                    countdown -= 1;
                    self.waiting_countdown.store(countdown, Ordering::Relaxed);
                }

                if countdown == 0 {
                    if sulfur_state == PotentSulfurState::Dormant {
                        let mut new_props = props;
                        new_props.r#potent_sulfur_state = PotentSulfurState::Erupting;
                        world.set_block_state(
                            &self.position,
                            new_props.to_state_id(&Block::POTENT_SULFUR),
                            BlockFlags::NOTIFY_ALL,
                        );
                        world.play_sound(
                            Sound::BlockPotentSulfurGeyserEruption,
                            SoundCategory::Blocks,
                            &self.position.to_f64(),
                        );
                        broadcast_vibration(world, &self.position, 10);
                    } else {
                        let mut new_props = props;
                        new_props.r#potent_sulfur_state = PotentSulfurState::Dormant;
                        world.set_block_state(
                            &self.position,
                            new_props.to_state_id(&Block::POTENT_SULFUR),
                            BlockFlags::NOTIFY_ALL,
                        );
                        broadcast_vibration(world, &self.position, 11);
                    }
                }
            }
        }

        // 3. Launch entity ticker (every tick, for Erupting and Continuous states)
        if sulfur_state == PotentSulfurState::Erupting || sulfur_state == PotentSulfurState::Continuous {
            if let Some(source_pos) = find_noxious_gas_source_block(world, self.position) {
                let water_blocks = (source_pos.0.y - self.position.0.y - 1).max(1);
                let geyser_force_height = 6 * water_blocks;

                let min = Vector3::new(
                    self.position.0.x as f64,
                    (self.position.0.y + 1) as f64,
                    self.position.0.z as f64,
                );
                let max = Vector3::new(
                    (self.position.0.x + 1) as f64,
                    (self.position.0.y + 1 + geyser_force_height) as f64,
                    (self.position.0.z + 1) as f64,
                );
                let launch_aabb = BoundingBox { min, max };
                let max_upward_v = 0.3 + (water_blocks as f64) * 0.1;

                for player in world.get_players_at_box(&launch_aabb) {
                    let is_flying = player.abilities.lock().unwrap_or_else(std::sync::PoisonError::into_inner).flying;
                    let is_spectator = player.gamemode.load() == GameMode::Spectator;
                    if !is_flying && !is_spectator {
                        let vel = player.living_entity.entity.velocity.load();
                        if vel.y < max_upward_v {
                            player
                                .living_entity
                                .entity
                                .add_velocity(Vector3::new(0.0, 0.2, 0.0));
                        }
                    }
                }

                for entity in world.get_entities_at_box(&launch_aabb) {
                    let base = entity.get_entity();
                    let vel = base.velocity.load();
                    if vel.y < max_upward_v {
                        base.add_velocity(Vector3::new(0.0, 0.2, 0.0));
                    }
                }
            }
        }
    }
}

impl PotentSulfurBlockEntity {
    pub const ID: &'static str = "minecraft:potent_sulfur";

    #[must_use]
    pub const fn new(position: BlockPos) -> Self {
        Self {
            position,
            waiting_countdown: AtomicI32::new(-1),
        }
    }

    pub fn reset_countdown(&self) {
        self.waiting_countdown.store(-1, Ordering::Relaxed);
    }
}

fn find_noxious_gas_source_block(world: &World, origin: BlockPos) -> Option<BlockPos> {
    let max_y = origin.0.y + 4 + 1;
    let mut y = origin.0.y + 1;
    while y <= max_y {
        let current_pos = BlockPos::new(origin.0.x, y, origin.0.z);
        let block_state = world.get_block_state(&current_pos);
        let is_water = block_state.id.to_block_id() == BlockId::WATER;
        if !is_water {
            if block_state.is_air() {
                return Some(current_pos);
            }
            break;
        }
        y += 1;
    }
    None
}

fn broadcast_vibration(world: &Arc<World>, pos: &BlockPos, frequency: u8) {
    let r = 8;
    for dx in -r..=r {
        for dy in -r..=r {
            for dz in -r..=r {
                let target = BlockPos::new(pos.0.x + dx, pos.0.y + dy, pos.0.z + dz);
                let block = world.get_block(&target);
                if block.id == BlockId::SCULK_SENSOR || block.id == BlockId::CALIBRATED_SCULK_SENSOR {
                    crate::block::blocks::redstone::sculk_sensor::SculkSensorBlock::trigger(
                        world,
                        &target,
                        block,
                        frequency,
                    );
                }
            }
        }
    }
}
