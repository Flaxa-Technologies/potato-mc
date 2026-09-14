use std::sync::{
    Arc, Mutex, Weak,
    atomic::{AtomicBool, AtomicI32, Ordering},
};

use pumpkin_data::Block;
use pumpkin_data::attributes::Attributes;
use pumpkin_data::damage::DamageType;
use pumpkin_data::entity::{EntityPose, EntityType};
use pumpkin_data::sound::{Sound, SoundCategory};
use pumpkin_data::tag;
use pumpkin_nbt::compound::NbtCompound;
use pumpkin_util::math::position::BlockPos;
use pumpkin_util::math::vector3::Vector3;

use crate::entity::{
    Entity, EntityBase,
    ai::goal::{
        active_target::ActiveTargetGoal, look_around::RandomLookAroundGoal,
        look_at_entity::LookAtEntityGoal, swim::SwimGoal, wander_around::WanderAroundGoal,
    },
    mob::{Mob, MobEntity},
    projectile::{
        ThrownItemEntity,
        wind_charge::{WIND_CHARGE_GRAVITY, WindChargeEntity},
    },
};
use crate::world::World;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BreezeState {
    Standing,
    PreparingShoot { ticks: i32, target_id: i32 },
    ShootingRecover { ticks: i32, target_id: i32 },
    PreparingJump { ticks: i32, target_pos: BlockPos },
    LongJumping { ticks: i32 },
    Sliding { ticks: i32, target: Vector3<f64> },
}

pub struct BreezeEntity {
    pub mob_entity: MobEntity,
    pub whirl_sound_timer: AtomicI32,
    pub ambient_sound_timer: AtomicI32,
    pub jump_cooldown: AtomicI32,
    pub shoot_cooldown: AtomicI32,
    pub state: Mutex<BreezeState>,
}

impl BreezeEntity {
    pub fn new(entity: Entity) -> Arc<Self> {
        let mob_entity = MobEntity::new(entity);
        let breeze = Self {
            mob_entity,
            whirl_sound_timer: AtomicI32::new(rand::random_range(1..=80)),
            ambient_sound_timer: AtomicI32::new(rand::random_range(80..160)),
            jump_cooldown: AtomicI32::new(10),
            shoot_cooldown: AtomicI32::new(10),
            state: Mutex::new(BreezeState::Standing),
        };
        let mob_arc = Arc::new(breeze);
        let mob_weak: Weak<dyn Mob> = {
            let mob_arc: Arc<dyn Mob> = mob_arc.clone();
            Arc::downgrade(&mob_arc)
        };

        // Initialize attributes per vanilla 26.2
        {
            let mut attributes = mob_arc
                .mob_entity
                .living_entity
                .attributes
                .write()
                .unwrap_or_else(std::sync::PoisonError::into_inner);

            if let Some(speed) = attributes.get_mut(&Attributes::MOVEMENT_SPEED.id) {
                speed.base_value = 0.63;
                speed.dirty.store(true, Ordering::Relaxed);
            }
            if let Some(health) = attributes.get_mut(&Attributes::MAX_HEALTH.id) {
                health.base_value = 30.0;
                health.dirty.store(true, Ordering::Relaxed);
            }
            if let Some(follow) = attributes.get_mut(&Attributes::FOLLOW_RANGE.id) {
                follow.base_value = 24.0;
                follow.dirty.store(true, Ordering::Relaxed);
            }
            if let Some(damage) = attributes.get_mut(&Attributes::ATTACK_DAMAGE.id) {
                damage.base_value = 3.0;
                damage.dirty.store(true, Ordering::Relaxed);
            }
        }

        {
            let mut goal_selector = mob_arc
                .mob_entity
                .goals_selector
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);

            goal_selector.add_goal(0, Box::new(SwimGoal::default()));
            goal_selector.add_goal(5, Box::new(WanderAroundGoal::new(0.6)));
            goal_selector.add_goal(
                6,
                LookAtEntityGoal::with_default(mob_weak.clone(), &EntityType::PLAYER, 8.0),
            );
            goal_selector.add_goal(7, Box::new(RandomLookAroundGoal::default()));

            let mut target_selector = mob_arc
                .mob_entity
                .target_selector
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            target_selector.add_goal(
                1,
                ActiveTargetGoal::with_default(&mob_arc.mob_entity, &EntityType::PLAYER, true),
            );
            target_selector.add_goal(
                2,
                ActiveTargetGoal::with_default(&mob_arc.mob_entity, &EntityType::IRON_GOLEM, true),
            );
        }

        mob_arc
    }

    pub fn play_whirl_sound(&self) {
        let entity = &self.mob_entity.living_entity.entity;
        let world = entity.world.load();
        let pitch = 0.7 + 0.4 * rand::random::<f32>();
        let volume = 0.8 + 0.2 * rand::random::<f32>();
        world.play_sound_fine(
            Sound::EntityBreezeWhirl,
            SoundCategory::Hostile,
            &entity.pos.load(),
            volume,
            pitch,
        );
    }
}

impl Mob for BreezeEntity {
    fn get_mob_entity(&self) -> &MobEntity {
        &self.mob_entity
    }

    fn can_attack(&self, target: &dyn EntityBase) -> bool {
        let entity_type = target.get_entity().entity_type;
        entity_type == &EntityType::PLAYER || entity_type == &EntityType::IRON_GOLEM
    }

    fn mob_tick(&self, _caller: &dyn EntityBase) {
        let entity = &self.mob_entity.living_entity.entity;
        if !entity.is_alive() {
            return;
        }

        let world = entity.world.load();

        // 1. Whirl ambient sound
        let whirl = self.whirl_sound_timer.fetch_sub(1, Ordering::Relaxed) - 1;
        if whirl <= 0 {
            self.play_whirl_sound();
            self.whirl_sound_timer
                .store(rand::random_range(1..=80), Ordering::Relaxed);
        }

        // 2. Idle ambient sound (when no target or in air)
        let ambient = self.ambient_sound_timer.fetch_sub(1, Ordering::Relaxed) - 1;
        if ambient <= 0 {
            let sound = if entity.on_ground.load(Ordering::Relaxed) {
                Sound::EntityBreezeIdleGround
            } else {
                Sound::EntityBreezeIdleAir
            };
            world.play_sound_fine(
                sound,
                SoundCategory::Hostile,
                &entity.pos.load(),
                1.0,
                1.0,
            );
            self.ambient_sound_timer
                .store(rand::random_range(80..160), Ordering::Relaxed);
        }

        // 3. Cooldown decrement
        if self.jump_cooldown.load(Ordering::Relaxed) > 0 {
            self.jump_cooldown.fetch_sub(1, Ordering::Relaxed);
        }
        if self.shoot_cooldown.load(Ordering::Relaxed) > 0 {
            self.shoot_cooldown.fetch_sub(1, Ordering::Relaxed);
        }

        // 4. State machine execution
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        match *state {
            BreezeState::Standing => {
                entity.set_pose(EntityPose::Standing);

                if let Some(target) = self.mob_entity.get_target() {
                    let target_pos = target.get_entity().pos.load();
                    let cur_pos = entity.pos.load();
                    let diff = target_pos - cur_pos;
                    let dist = (diff.x * diff.x + diff.z * diff.z).sqrt();

                    if dist <= 24.0 {
                        // Inner ring check (<= 4 blocks): slide away
                        if dist <= 4.0 && entity.on_ground.load(Ordering::Relaxed) {
                            let away_pos = random_point_away_from_enemy(cur_pos, target_pos);
                            if let Some(surface_pos) = snap_to_surface(&world, away_pos) {
                                let surface_v3 = Vector3::new(
                                    surface_pos.0.x as f64 + 0.5,
                                    surface_pos.0.y as f64,
                                    surface_pos.0.z as f64 + 0.5,
                                );
                                entity.set_pose(EntityPose::Sliding);
                                world.play_sound_fine(
                                    Sound::EntityBreezeSlide,
                                    SoundCategory::Hostile,
                                    &cur_pos,
                                    1.0,
                                    1.0,
                                );
                                *state = BreezeState::Sliding {
                                    ticks: 20,
                                    target: surface_v3,
                                };
                                return;
                            }
                        }

                        // Shoot check: if within range (<= 16 blocks) and shoot cooldown expired
                        if self.shoot_cooldown.load(Ordering::Relaxed) <= 0 && dist <= 16.0 {
                            entity.set_pose(EntityPose::Inhaling);
                            world.play_sound_fine(
                                Sound::EntityBreezeInhale,
                                SoundCategory::Hostile,
                                &cur_pos,
                                1.0,
                                1.0,
                            );
                            *state = BreezeState::PreparingShoot {
                                ticks: 15,
                                target_id: target.get_entity().entity_id,
                            };
                            return;
                        }

                        // Long jump check: if jump cooldown expired and clearance above
                        let cur_block_pos = BlockPos::new(
                            cur_pos.x.floor() as i32,
                            cur_pos.y.floor() as i32,
                            cur_pos.z.floor() as i32,
                        );
                        if self.jump_cooldown.load(Ordering::Relaxed) <= 0
                            && dist > 4.0
                            && can_jump_from_current_pos(&world, &cur_block_pos)
                        {
                            let behind = random_point_behind_target(
                                target_pos,
                                target.get_entity().yaw.load(),
                            );
                            if let Some(jump_target) = snap_to_surface(&world, behind) {
                                entity.set_pose(EntityPose::Inhaling);
                                world.play_sound_fine(
                                    Sound::EntityBreezeCharge,
                                    SoundCategory::Hostile,
                                    &cur_pos,
                                    1.0,
                                    1.0,
                                );
                                *state = BreezeState::PreparingJump {
                                    ticks: 10,
                                    target_pos: jump_target,
                                };
                                return;
                            }
                        }

                        // Slide reposition check
                        if entity.on_ground.load(Ordering::Relaxed) {
                            let slide_dest =
                                random_point_in_middle_circle(cur_pos, target_pos);
                            if let Some(surface_pos) = snap_to_surface(&world, slide_dest) {
                                let surface_v3 = Vector3::new(
                                    surface_pos.0.x as f64 + 0.5,
                                    surface_pos.0.y as f64,
                                    surface_pos.0.z as f64 + 0.5,
                                );
                                entity.set_pose(EntityPose::Sliding);
                                world.play_sound_fine(
                                    Sound::EntityBreezeSlide,
                                    SoundCategory::Hostile,
                                    &cur_pos,
                                    1.0,
                                    1.0,
                                );
                                *state = BreezeState::Sliding {
                                    ticks: 30,
                                    target: surface_v3,
                                };
                            }
                        }
                    }
                }
            }

            BreezeState::PreparingShoot { ticks, target_id } => {
                if let Some(target) = world.get_entity_by_id(target_id) {
                    let target_pos = target.get_entity().pos.load();
                    let cur_pos = entity.pos.load();
                    let dx = target_pos.x - cur_pos.x;
                    let dz = target_pos.z - cur_pos.z;
                    let yaw = (-dx).atan2(dz).to_degrees() as f32;
                    let pitch = (-(target_pos.y - cur_pos.y))
                        .atan2((dx * dx + dz * dz).sqrt())
                        .to_degrees() as f32;
                    entity.set_rotation(yaw, pitch);

                    if ticks <= 0 {
                        let spawn_pos = Vector3::new(cur_pos.x, cur_pos.y + 1.185, cur_pos.z);
                        let thrown = ThrownItemEntity {
                            entity: Entity::from_uuid(
                                uuid::Uuid::new_v4(),
                                world.clone(),
                                spawn_pos,
                                &EntityType::BREEZE_WIND_CHARGE,
                            ),
                            owner_id: Some(entity.entity_id),
                            collides_with_projectiles: false,
                            has_hit: AtomicBool::new(false),
                            gravity: WIND_CHARGE_GRAVITY,
                        };
                        let wind_charge = WindChargeEntity::new_breeze(thrown);
                        let target_ent = target.get_entity();
                        let target_height = f64::from(target_ent.entity_dimension.load().height);
                        let progress = if target_ent.vehicle.lock().map(|v| v.is_some()).unwrap_or(false) { 0.8 } else { 0.3 };
                        let target_y = target_pos.y + target_height * progress;
                        let xd = target_pos.x - spawn_pos.x;
                        let yd = target_y - spawn_pos.y;
                        let zd = target_pos.z - spawn_pos.z;
                        let difficulty = world.level_info.load().difficulty as i32;
                        let uncertainty = (5 - difficulty * 4).max(0) as f64;
                        wind_charge.set_velocity(xd, yd, zd, 0.7, uncertainty);
                        world.spawn_entity_non_save(Arc::new(wind_charge));

                        world.play_sound_fine(
                            Sound::EntityBreezeShoot,
                            SoundCategory::Hostile,
                            &cur_pos,
                            1.5,
                            1.0,
                        );

                        entity.set_pose(EntityPose::Shooting);
                        *state = BreezeState::ShootingRecover {
                            ticks: 4,
                            target_id,
                        };
                    } else {
                        *state = BreezeState::PreparingShoot {
                            ticks: ticks - 1,
                            target_id,
                        };
                    }
                } else {
                    entity.set_pose(EntityPose::Standing);
                    *state = BreezeState::Standing;
                }
            }

            BreezeState::ShootingRecover { ticks, target_id } => {
                if ticks <= 0 {
                    entity.set_pose(EntityPose::Standing);
                    self.shoot_cooldown.store(10, Ordering::Relaxed);
                    *state = BreezeState::Standing;
                } else {
                    *state = BreezeState::ShootingRecover {
                        ticks: ticks - 1,
                        target_id,
                    };
                }
            }

            BreezeState::PreparingJump { ticks, target_pos } => {
                let cur_pos = entity.pos.load();
                let target_v3 = Vector3::new(
                    target_pos.0.x as f64 + 0.5,
                    target_pos.0.y as f64,
                    target_pos.0.z as f64 + 0.5,
                );
                let dx = target_v3.x - cur_pos.x;
                let dz = target_v3.z - cur_pos.z;
                let yaw = (-dx).atan2(dz).to_degrees() as f32;
                entity.set_rotation(yaw, entity.pitch.load());

                if ticks <= 0 {
                    if let Some(launch_vel) = calculate_optimal_jump_vector(cur_pos, target_v3, 24.0) {
                        entity.velocity.store(launch_vel);
                        entity.velocity_dirty.store(true, Ordering::Relaxed);
                        entity.set_pose(EntityPose::LongJumping);
                        world.play_sound_fine(
                            Sound::EntityBreezeJump,
                            SoundCategory::Hostile,
                            &cur_pos,
                            1.0,
                            1.0,
                        );
                        *state = BreezeState::LongJumping { ticks: 60 };
                    } else {
                        entity.set_pose(EntityPose::Standing);
                        *state = BreezeState::Standing;
                    }
                } else {
                    *state = BreezeState::PreparingJump {
                        ticks: ticks - 1,
                        target_pos,
                    };
                }
            }

            BreezeState::LongJumping { ticks } => {
                let on_ground = entity.on_ground.load(Ordering::Relaxed);
                let in_water = entity.touching_water.load(Ordering::SeqCst);

                if on_ground || in_water || ticks <= 0 {
                    world.play_sound_fine(
                        Sound::EntityBreezeLand,
                        SoundCategory::Hostile,
                        &entity.pos.load(),
                        1.0,
                        1.0,
                    );
                    entity.set_pose(EntityPose::Standing);
                    self.jump_cooldown.store(10, Ordering::Relaxed);
                    self.shoot_cooldown.store(0, Ordering::Relaxed);
                    *state = BreezeState::Standing;
                } else {
                    *state = BreezeState::LongJumping { ticks: ticks - 1 };
                }
            }

            BreezeState::Sliding { ticks, target } => {
                let cur_pos = entity.pos.load();
                let diff = target - cur_pos;
                let dist_sq = diff.x * diff.x + diff.z * diff.z;

                if dist_sq < 0.5 * 0.5 || ticks <= 0 {
                    entity.set_pose(EntityPose::Standing);
                    self.shoot_cooldown.store(0, Ordering::Relaxed);
                    *state = BreezeState::Standing;
                } else {
                    let len = dist_sq.sqrt();
                    let speed = 0.38;
                    let vx = (diff.x / len) * speed;
                    let vz = (diff.z / len) * speed;
                    let cur_vel = entity.velocity.load();
                    entity.velocity.store(Vector3::new(vx, cur_vel.y, vz));
                    entity.velocity_dirty.store(true, Ordering::Relaxed);
                    let yaw = (-diff.x).atan2(diff.z).to_degrees() as f32;
                    entity.set_rotation(yaw, entity.pitch.load());
                    *state = BreezeState::Sliding {
                        ticks: ticks - 1,
                        target,
                    };
                }
            }
        }
    }

    fn pre_damage(&self, damage_type: DamageType, source: Option<&dyn EntityBase>) -> bool {
        // Invulnerable to damage from other Breezes per vanilla
        if let Some(src) = source {
            if *src.get_entity().entity_type == EntityType::BREEZE {
                return false;
            }
        }

        // Deflect all projectiles except wind charges
        if is_projectile_damage(damage_type) {
            let is_wind_charge = damage_type == DamageType::WIND_CHARGE
                || source.is_some_and(|s| {
                    *s.get_entity().entity_type == EntityType::WIND_CHARGE
                        || *s.get_entity().entity_type == EntityType::BREEZE_WIND_CHARGE
                });

            if !is_wind_charge {
                let entity = &self.mob_entity.living_entity.entity;
                let world = entity.world.load();
                world.play_sound_fine(
                    Sound::EntityBreezeDeflect,
                    SoundCategory::Hostile,
                    &entity.pos.load(),
                    1.0,
                    1.0,
                );

                if let Some(src) = source {
                    let vel = src.get_entity().velocity.load();
                    src.get_entity()
                        .velocity
                        .store(vel.multiply(-1.0, -1.0, -1.0));
                    src.get_entity()
                        .velocity_dirty
                        .store(true, Ordering::Relaxed);
                }
                return false;
            }
        }

        // When hurt, jump cooldown reduced to 2 ticks per vanilla JUMP_COOLDOWN_WHEN_HURT_TICKS
        self.jump_cooldown.store(2, Ordering::Relaxed);

        true
    }

    fn mob_write_nbt(&self, nbt: &mut NbtCompound) {
        nbt.put_int(
            "WhirlSoundTimer",
            self.whirl_sound_timer.load(Ordering::Relaxed),
        );
        nbt.put_int("JumpCooldown", self.jump_cooldown.load(Ordering::Relaxed));
        nbt.put_int(
            "ShootCooldown",
            self.shoot_cooldown.load(Ordering::Relaxed),
        );
    }

    fn mob_read_nbt(&self, nbt: &NbtCompound) {
        if let Some(val) = nbt.get_int("WhirlSoundTimer") {
            self.whirl_sound_timer.store(val, Ordering::Relaxed);
        }
        if let Some(val) = nbt.get_int("JumpCooldown") {
            self.jump_cooldown.store(val, Ordering::Relaxed);
        }
        if let Some(val) = nbt.get_int("ShootCooldown") {
            self.shoot_cooldown.store(val, Ordering::Relaxed);
        }
    }
}

fn is_projectile_damage(dt: DamageType) -> bool {
    let (names, _) = tag::DamageType::MINECRAFT_IS_PROJECTILE;
    names.contains(&dt.message_id)
}

fn snap_to_surface(world: &World, target: Vector3<f64>) -> Option<BlockPos> {
    let base_x = target.x.floor() as i32;
    let base_y = target.y.floor() as i32;
    let base_z = target.z.floor() as i32;

    for dy in 0..=10 {
        let pos = BlockPos::new(base_x, base_y - dy, base_z);
        let state = world.get_block_state(&pos);
        if !state.is_air() {
            let above = BlockPos::new(base_x, base_y - dy + 1, base_z);
            if world.get_block_state(&above).is_air() {
                return Some(above);
            }
        }
    }
    for dy in 1..=10 {
        let pos = BlockPos::new(base_x, base_y + dy, base_z);
        let state = world.get_block_state(&pos);
        if !state.is_air() {
            let above = BlockPos::new(base_x, base_y + dy + 1, base_z);
            if world.get_block_state(&above).is_air() {
                return Some(above);
            }
        }
    }
    None
}

fn can_jump_from_current_pos(world: &World, current_pos: &BlockPos) -> bool {
    let current_block = world.get_block(current_pos);
    if current_block == &Block::HONEY_BLOCK {
        return false;
    }
    for i in 1..=4 {
        let offset = BlockPos::new(current_pos.0.x, current_pos.0.y + i, current_pos.0.z);
        let state = world.get_block_state(&offset);
        if !state.is_air() && world.get_block(&offset) != &Block::WATER {
            return false;
        }
    }
    true
}

fn random_point_behind_target(enemy_pos: Vector3<f64>, enemy_yaw: f32) -> Vector3<f64> {
    let offset_angle = (rand::random::<f32>() - 0.5) * 90.0;
    let view_angle = (enemy_yaw + 180.0 + offset_angle).to_radians();
    let r = 4.0 + rand::random::<f64>() * 4.0;
    let dx = -f64::from(view_angle.sin()) * r;
    let dz = f64::from(view_angle.cos()) * r;
    Vector3::new(enemy_pos.x + dx, enemy_pos.y, enemy_pos.z + dz)
}

fn random_point_in_middle_circle(breeze_pos: Vector3<f64>, enemy_pos: Vector3<f64>) -> Vector3<f64> {
    let diff = enemy_pos - breeze_pos;
    let len = (diff.x * diff.x + diff.z * diff.z).sqrt();
    let r = 4.0 + rand::random::<f64>() * 4.0;
    let angle = rand::random::<f64>() * 2.0 * std::f64::consts::PI;
    if len > 0.1 {
        Vector3::new(
            enemy_pos.x + angle.cos() * r,
            enemy_pos.y,
            enemy_pos.z + angle.sin() * r,
        )
    } else {
        Vector3::new(
            breeze_pos.x + angle.cos() * r,
            breeze_pos.y,
            breeze_pos.z + angle.sin() * r,
        )
    }
}

fn random_point_away_from_enemy(breeze_pos: Vector3<f64>, enemy_pos: Vector3<f64>) -> Vector3<f64> {
    let diff = breeze_pos - enemy_pos;
    let len = (diff.x * diff.x + diff.z * diff.z).sqrt();
    let dist = 5.0;
    if len > 0.1 {
        Vector3::new(
            breeze_pos.x + (diff.x / len) * dist,
            breeze_pos.y,
            breeze_pos.z + (diff.z / len) * dist,
        )
    } else {
        let angle = rand::random::<f64>() * 2.0 * std::f64::consts::PI;
        Vector3::new(
            breeze_pos.x + angle.cos() * dist,
            breeze_pos.y,
            breeze_pos.z + angle.sin() * dist,
        )
    }
}

fn calculate_optimal_jump_vector(
    start: Vector3<f64>,
    target_pos: Vector3<f64>,
    follow_range: f64,
) -> Option<Vector3<f64>> {
    let max_jump_velocity = 0.058_333_334 * follow_range;
    let dx = target_pos.x - start.x;
    let dy = target_pos.y - start.y;
    let dz = target_pos.z - start.z;
    let r2 = dx * dx + dz * dz;
    let r = r2.sqrt();
    if r < 0.1 {
        return None;
    }
    let g = 0.08;
    let xz_ang = dz.atan2(dx);

    let mut angles = [40.0f64, 55.0, 60.0, 75.0, 80.0];
    for i in (1..angles.len()).rev() {
        let j = rand::random_range(0..=i);
        angles.swap(i, j);
    }

    for &angle_deg in &angles {
        let angrad = angle_deg.to_radians();
        let sin2ang = (2.0 * angrad).sin();
        let cosang = angrad.cos();
        let cosangsqr = cosang * cosang;
        let denom = r * sin2ang - 2.0 * dy * cosangsqr;
        if denom <= 0.0 {
            continue;
        }
        let v0sqr = (r2 * g) / denom;
        if v0sqr < 0.0 {
            continue;
        }
        let v0 = v0sqr.sqrt();
        if v0 > max_jump_velocity {
            continue;
        }
        let v0r = v0 * cosang;
        let v0y = v0 * angrad.sin();
        let vx = v0r * xz_ang.cos() * 0.95;
        let vy = v0y * 0.95;
        let vz = v0r * xz_ang.sin() * 0.95;
        return Some(Vector3::new(vx, vy, vz));
    }
    None
}
