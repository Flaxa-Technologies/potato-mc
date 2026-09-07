use std::sync::Arc;

use pumpkin_data::entity::MobCategory;
use pumpkin_util::math::vector2::Vector2;
use pumpkin_util::Difficulty;
use rand::{RngExt, rng};
use rand::seq::SliceRandom;
use rayon::prelude::*;
use std::sync::atomic::Ordering::Relaxed;
use tracing::{debug, trace, warn};

use pumpkin_world::chunk::ChunkData;

use crate::entity::EntityBase;
use crate::server::Server;
use crate::world::natural_spawner::{SpawnState, spawn_for_chunk};
use pumpkin_util::math::get_section_cord;
use pumpkin_util::math::vector3::Vector3;

use super::World;

impl World {
    #[expect(clippy::too_many_lines)]
    pub fn tick(self: &Arc<Self>, server: &Arc<Server>) {
        const ENTITY_TICK_BATCH_SIZE: usize = 16;

        let start = std::time::Instant::now();

        self.flush_block_updates();
        self.flush_synced_block_events();
        self.update_active_chunks();
        self.tick_environment();
        let mut raids = {
            let mut guard = self
                .raids
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            std::mem::take(&mut *guard)
        };
        raids.tick(self);
        {
            let mut guard = self
                .raids
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            for (id, raid) in guard.raid_map.drain() {
                raids.raid_map.insert(id, raid);
            }
            raids.next_id = raids.next_id.max(guard.next_id);
            *guard = raids;
        };

        let t_chunks = std::time::Instant::now();
        self.tick_chunks(server);
        let chunk_elapsed = t_chunks.elapsed();

        let handle = server.runtime.clone();

        let players = self.players.load();
        let player_count = players.len();
        let players_cache: Vec<_> = players
            .par_iter()
            .map(|player| {
                let entity = player.get_entity();
                let pos = entity.pos.load();
                let bb = entity.bounding_box.load().expand(1.0, 0.5, 1.0);
                let chunk_pos = Vector2::new(
                    get_section_cord(pos.x.floor() as i32),
                    get_section_cord(pos.z.floor() as i32),
                );
                (player, pos, bb, chunk_pos)
            })
            .collect();

        let t_players = std::time::Instant::now();
        let player_handle = handle.clone();
        players.par_iter().for_each(|player| {
            let _guard = player_handle.enter();
            player.tick(server);
        });
        let player_elapsed = t_players.elapsed();

        let entities_to_tick = self.entities.load();
        let entity_count = entities_to_tick.len();
        let active_chunks = self
            .active_chunks
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let level_for_entities = self.level.clone();
        let entity_handle = handle.clone();

        let t_entities = std::time::Instant::now();
        let tickable: Vec<_> = entities_to_tick
            .par_iter()
            .filter_map(|entity| {
                if entity.get_entity().is_removed() {
                    return None;
                }
                let entity_pos = entity.get_entity().pos.load();
                let entity_chunk = Vector2::new(
                    get_section_cord(entity_pos.x.floor() as i32),
                    get_section_cord(entity_pos.z.floor() as i32),
                );
                if !active_chunks.contains(&entity_chunk) {
                    return None;
                }
                if !level_for_entities.is_chunk_loaded(&entity_chunk) {
                    return None;
                }
                Some((entity, entity_chunk))
            })
            .collect();

        let server_ref = server.as_ref();
        tickable
            .par_chunks(ENTITY_TICK_BATCH_SIZE)
            .for_each(|batch| {
                let _guard = entity_handle.enter();

                for (entity, entity_chunk) in batch {
                    if entity.get_entity().is_removed() {
                        continue;
                    }
                    entity.get_entity().age.fetch_add(1, Relaxed);
                    entity.tick(entity.as_ref(), server_ref);

                    let entity_inner = entity.get_entity();
                    let entity_pos = entity_inner.pos.load();
                    let entity_bb = entity_inner.bounding_box.load();

                    for (player, player_pos, player_bb, player_chunk) in &players_cache {
                        if (player_chunk.x - entity_chunk.x).abs() <= 1
                            && (player_chunk.y - entity_chunk.y).abs() <= 1
                            && (player_pos.x - entity_pos.x).abs() < 5.0
                            && (player_pos.y - entity_pos.y).abs() < 5.0
                            && (player_pos.z - entity_pos.z).abs() < 5.0
                            && player_bb.intersects(&entity_bb)
                        {
                            entity.on_player_collision(player);
                            break;
                        }
                    }
                }
            });
        let entity_elapsed = t_entities.elapsed();

        self.entity_tracker.update_all(self);

        let mut block_entities: Vec<Arc<dyn crate::block::entities::BlockEntity>> = Vec::new();
        for chunk_pos in active_chunks.iter() {
            self.migrate_pending_block_entities(*chunk_pos);
            if let Some(chunk_block_entities) = self.block_entities.get(chunk_pos) {
                block_entities.extend(chunk_block_entities.values().cloned());
            }
        }
        let block_entity_count = block_entities.len();

        let t_be = std::time::Instant::now();
        let be_handle = handle;
        block_entities.par_chunks(16).for_each(|batch| {
            let _guard = be_handle.enter();
            for be in batch {
                be.tick(self);
            }
        });
        let block_entity_elapsed = t_be.elapsed();

        self.level
            .chunk_loading
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .send_change();

        if let Some(ref fight_mutex) = self.dragon_fight {
            crate::world::dragon_fight::DragonFight::tick(fight_mutex, self);
        }

        let total_elapsed = start.elapsed();
        if total_elapsed.as_millis() > 50 {
            debug!(
                "Slow Tick [{}ms]: Chunks: {:?} | Players({}): {:?} | Entities({}): {:?} | Block Entities({}): {:?}",
                total_elapsed.as_millis(),
                chunk_elapsed,
                player_count,
                player_elapsed,
                entity_count,
                entity_elapsed,
                block_entity_count,
                block_entity_elapsed,
            );
        }
    }

    pub fn tick_environment(self: &Arc<Self>) {
        let (world_age, is_night, time_of_day) = {
            let mut level_time = self
                .level_time
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let advance_time = self.level_info.load().game_rules.advance_time;
            level_time.tick(advance_time);

            // Auto-save logic
            if level_time.world_age % 100 == 0 {
                self.level.should_unload.store(true, Relaxed);
                let cleaned_chunks = self.level.clean_memory();
                if !cleaned_chunks.is_empty() {
                    let world_clone = self.clone();
                    if let Some(server) = self.server.upgrade() {
                        server.spawn_task(async move {
                            world_clone.remove_entities_in_chunks(&cleaned_chunks).await;
                            world_clone.level.clean_entity_chunks(&cleaned_chunks);
                        });
                    }
                }
                // If autosave is configured and this tick will trigger an autosave, don't double notify
                if self.level.autosave_ticks == 0 {
                    self.level.level_channel.notify();
                } else {
                    let autosave = self.level.autosave_ticks as i64;
                    if autosave == 0 || level_time.world_age % autosave != 0 {
                        self.level.level_channel.notify();
                    }
                }
            }
            if self.level.autosave_ticks > 0 && self.level.save_enabled.load(Relaxed) {
                let autosave = self.level.autosave_ticks as i64;
                if autosave > 0 && level_time.world_age % autosave == 0 {
                    self.level.should_save.store(true, Relaxed);
                    self.level.level_channel.notify();
                }
            }
            (
                level_time.world_age,
                level_time.is_night(),
                level_time.time_of_day,
            )
        };

        let (should_reset_weather, weather_cycle_enabled) = {
            let mut weather = self
                .weather
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            weather.tick_weather(self);
            (
                weather.raining || weather.thundering,
                weather.weather_cycle_enabled,
            )
        };

        if self.should_skip_night() && is_night {
            let level_time = {
                let mut guard = self
                    .level_time
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                let time = time_of_day + 24000;
                guard.set_time(time - time % 24000);
                guard.clone()
            };
            level_time.send_time(self);

            for player in self.players.load().iter() {
                player.wake_up();
            }

            if weather_cycle_enabled && should_reset_weather {
                let mut weather = self
                    .weather
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                weather.reset_weather_cycle(self);
            }
        } else if world_age % 20 == 0 {
            let level_time = self
                .level_time
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .clone();
            level_time.send_time(self);
        }
    }

    #[expect(clippy::too_many_lines)]
    pub fn tick_chunks(self: &Arc<Self>, server: &Arc<Server>) {
        const BATCH_SIZE: usize = 32;
        let random_tick_speed = self.level_info.load().game_rules.random_tick_speed;

        let active_chunks = self
            .active_chunks
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let tick_data = self.level.get_tick_data(&active_chunks, random_tick_speed);
        let handle = server.runtime.clone();

        // 1. Parallel Block Ticks via Rayon
        let world = self.clone();
        let block_handle = handle.clone();
        tick_data
            .block_ticks
            .par_chunks(BATCH_SIZE)
            .for_each(|batch| {
                let _guard = block_handle.enter();
                let world = world.clone();
                for scheduled_tick in batch {
                    let pos = scheduled_tick.position;
                    let block = world.get_block(&pos);
                    if let Some(pumpkin_block) = world.block_registry.get_pumpkin_block(block.id) {
                        pumpkin_block.on_scheduled_tick(crate::block::OnScheduledTickArgs {
                            world: &world,
                            block,
                            position: &pos,
                        });
                    }
                }
            });

        // 2. Parallel Fluid Ticks via Rayon
        let world = self.clone();
        let fluid_handle = handle.clone();
        tick_data
            .fluid_ticks
            .par_chunks(BATCH_SIZE)
            .for_each(|batch| {
                let _guard = fluid_handle.enter();
                let world = world.clone();
                for scheduled_tick in batch {
                    let pos = scheduled_tick.position;
                    let fluid = world.get_fluid(&pos);
                    if let Some(pumpkin_fluid) = world.block_registry.get_pumpkin_fluid(fluid.id) {
                        pumpkin_fluid.on_scheduled_tick(&world, fluid, &pos);
                    }
                }
            });

        // 3. Parallel Random Ticks via Rayon
        let world = self.clone();
        let random_handle = handle.clone();
        tick_data
            .random_ticks
            .par_chunks(BATCH_SIZE)
            .for_each(|batch| {
                let _guard = random_handle.enter();
                let world = world.clone();
                for scheduled_tick in batch {
                    let pos = scheduled_tick.position;
                    let (block, fluid) =
                        match (scheduled_tick.tick_block, scheduled_tick.tick_fluid) {
                            (true, true) => {
                                let (b, f) = world.get_block_and_fluid(&pos);
                                (Some(b), Some(f))
                            }
                            (true, false) => (Some(world.get_block(&pos)), None),
                            (false, true) => (None, Some(world.get_fluid(&pos))),
                            (false, false) => (None, None),
                        };

                    if let Some(block) = block
                        && let Some(pumpkin_block) =
                            world.block_registry.get_pumpkin_block(block.id)
                    {
                        pumpkin_block.random_tick(crate::block::RandomTickArgs {
                            world: &world,
                            block,
                            position: &pos,
                        });
                    }

                    if let Some(fluid) = fluid
                        && let Some(pumpkin_fluid) =
                            world.block_registry.get_pumpkin_fluid(fluid.id)
                    {
                        pumpkin_fluid.random_tick(fluid, &world, &pos);
                    }
                }
            });

        // 4. Calculate Spawn List (Sequential setup)
        let spawning_config = crate::spawning_config::SPAWNING_CONFIG.load();
        let cycle_ticks = i64::from(spawning_config.global.ticks_per_spawn_cycle.max(1));
        let should_tick_spawning = self.get_time_of_day() % cycle_ticks == 0;

        if should_tick_spawning {
            let spawn_state = self.spawn_state.load();
            let (spawn_mobs, spawn_monsters, peaceful) = {
                let lock = self.level_info.load();
                (
                    lock.game_rules.spawn_mobs,
                    lock.game_rules.spawn_monsters,
                    lock.difficulty == Difficulty::Peaceful,
                )
            };
            let spawn_passives = self.get_time_of_day() % 400 == 0;
            let spawn_enemies = !peaceful && spawn_monsters && spawn_mobs;
            let spawn_passives = spawn_passives && spawn_mobs;

            let spawn_list = Arc::new(crate::world::natural_spawner::get_filtered_spawning_categories(
                &spawn_state,
                spawn_mobs,
                spawn_enemies,
                spawn_passives,
            ));

            // 5. Parallel Chunk Spawners via Rayon
            if !spawn_list.is_empty() {
                use pumpkin_util::gamemode::GameMode;
                let radius = spawning_config.global.spawn_chunk_radius as i32;
                let radius_sq = radius * radius;
                let player_chunks: Vec<Vector2<i32>> = self
                    .players
                    .load()
                    .iter()
                    .filter_map(|p| {
                        if p.gamemode.load() != GameMode::Spectator {
                            let pos = p.position();
                            Some(Vector2::new(
                                get_section_cord(pos.x.floor() as i32),
                                get_section_cord(pos.z.floor() as i32),
                            ))
                        } else {
                            None
                        }
                    })
                    .collect();

                let mut spawning_chunks = Vec::new();
                for pos in active_chunks.iter() {
                    let within_radius = player_chunks.is_empty()
                        || player_chunks.iter().any(|pc| {
                            let dx = pos.x - pc.x;
                            let dz = pos.y - pc.y;
                            dx * dx + dz * dz <= radius_sq
                        });

                    if within_radius
                        && let Some(chunk) = self.level.read_chunk_sync(pos, std::clone::Clone::clone)
                    {
                        spawning_chunks.push((*pos, chunk));
                    }
                }

                spawning_chunks.shuffle(&mut rng());

                let world = self.clone();
                let spawn_handle = handle;
                spawning_chunks.par_chunks(8).for_each(|batch| {
                    let _guard = spawn_handle.enter();
                    let world = world.clone();
                    let s_list = spawn_list.clone();
                    let s_state = spawn_state.clone();
                    for (pos, chunk) in batch {
                        world.tick_spawning_chunk(*pos, chunk, &s_list, &s_state);
                    }
                });
            }
        }

        // Update chunk inhabited time for active chunks in parallel with Rayon
        let loaded_chunks = self.level.loaded_chunks.clone();
        let active_chunks_vec: Vec<_> = active_chunks.iter().copied().collect();
        active_chunks_vec.par_iter().for_each(|pos| {
            if let Some(chunk) = loaded_chunks.get(pos) {
                chunk.inhabited_time.fetch_add(1, Relaxed);
            }
        });
    }

    pub(crate) fn spawn_world_entity_chunks(
        self: &Arc<Self>,
        player: Arc<crate::entity::player::Player>,
        chunks: Vec<pumpkin_util::math::vector2::Vector2<i32>>,
        center_chunk: pumpkin_util::math::vector2::Vector2<i32>,
    ) {
        use pumpkin_data::entity::EntityType;
        use pumpkin_world::chunk::ChunkHeightmapType::MotionBlocking;
        use uuid::Uuid;

        #[cfg(debug_assertions)]
        let inst = std::time::Instant::now();

        // Sort such that the first chunks are closest to the center.
        let mut chunks = chunks;
        chunks.sort_unstable_by_key(|pos| {
            let rel_x = pos.x - center_chunk.x;
            let rel_z = pos.y - center_chunk.y;
            rel_x * rel_x + rel_z * rel_z
        });

        let mut entity_receiver = self.level.receive_entity_chunks(chunks);
        let level = self.level.clone();
        let world = self.clone();

        player.clone().spawn_task(async move {
            'main: loop {
                let recv_result = tokio::select! {
                    () = player.client.await_close_interrupt() => {
                        debug!("Canceling player packet processing");
                        None
                    },
                    recv_result = entity_receiver.recv() => {
                        recv_result
                    }
                };

                let Some((chunk_weak, first_load)) = recv_result else {
                    break;
                };

                let Some(chunk) = chunk_weak.upgrade() else {
                    continue;
                };

                let position = pumpkin_util::math::vector2::Vector2::new(chunk.x, chunk.z);

                if !level.is_chunk_watched(&position) {
                    trace!(
                        "Received entity chunk {:?}, but it is no longer watched; leaving it for the unload path",
                        &position
                    );
                    continue 'main;
                }

                if first_load {
                    let entity_nbts = std::mem::take(
                        &mut *chunk
                            .data
                            .lock()
                            .unwrap_or_else(std::sync::PoisonError::into_inner),
                    );
                    let mut entities_to_add: Vec<std::sync::Arc<dyn crate::entity::EntityBase>> =
                        Vec::with_capacity(entity_nbts.len());
                    for entity_nbt in &entity_nbts {
                        let Some(id) = entity_nbt.get_string("id") else {
                            debug!("Entity has no ID");
                            continue;
                        };
                        let Some(entity_type) =
                            EntityType::from_name(id.strip_prefix("minecraft:").unwrap_or(id))
                        else {
                            warn!("Entity has no valid Entity Type {id}");
                            continue;
                        };

                        let uuid = entity_nbt.get_uuid("UUID").unwrap_or_else(Uuid::new_v4);
                        let entity =
                            crate::entity::r#type::from_type(entity_type, Vector3::new(0.0, 0.0, 0.0), &world, uuid);
                        entity.read_nbt_non_mut(entity_nbt);
                        entity.init_data_tracker();

                        let base_entity = entity.get_entity();
                        base_entity.velocity.store(Vector3::default());

                        player.client.enqueue_spawn_packet(&entity);
                        player.try_restore_vehicle(&entity);
                        entities_to_add.push(entity);
                    }

                    if !entities_to_add.is_empty() {
                        world.entities.rcu(|current_entities| {
                            let mut new_entities = (**current_entities).clone();
                            new_entities.extend(entities_to_add.iter().cloned());
                            new_entities
                        });
                    }
                } else {
                    for entity in world.entities.load().iter() {
                        let base_entity = entity.get_entity();
                        if base_entity.chunk_pos.load() == position {
                            player.client.enqueue_spawn_packet(entity);
                            player.try_restore_vehicle(entity);
                        }
                    }
                }
            }

            #[cfg(debug_assertions)]
            debug!("Chunks queued after {}ms", inst.elapsed().as_millis());
        });
    }

    pub fn tick_spawning_chunk(
        self: &Arc<Self>,
        chunk_pos: Vector2<i32>,
        chunk: &Arc<ChunkData>,
        spawn_list: &Vec<&'static MobCategory>,
        spawn_state: &Arc<SpawnState>,
    ) {
        // this.level.tickThunder(chunk);
        //TODO check in simulation distance
        let (is_raining, is_thundering) = (self.is_raining(), self.is_thundering());

        if is_raining && is_thundering && rng().random_range(0..100_000) == 0 {
            let rand_value = rng().random::<i32>() >> 2;
            let delta = Vector3::new(rand_value & 15, rand_value >> 16 & 15, rand_value >> 8 & 15);
            use pumpkin_world::chunk::ChunkHeightmapType::MotionBlocking;
            let random_pos = Vector3::new(
                chunk_pos.x << 4,
                chunk
                    .heightmap
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .get(
                        MotionBlocking,
                        chunk_pos.x << 4,
                        chunk_pos.y << 4,
                        self.min_y,
                    ),
                chunk_pos.y << 4,
            )
            .add(&delta);
            // TODO this.getBrightness(LightLayer.SKY, blockPos) >= 15;
            // TODO heightmap

            // TODO findLightningRod(blockPos)
            // TODO encapsulatingFullBlocks
            if true {
                // TODO biome.getPrecipitationAt(pos, this.getSeaLevel()) == Biome.Precipitation.RAIN
                // TODO this.getCurrentDifficultyAt(blockPos);
                use pumpkin_data::entity::EntityType;
                if rng().random::<f32>() < 0.0675
                    && self.get_block(&random_pos.to_block_pos().down()) != &pumpkin_data::Block::LIGHTNING_ROD
                {
                    let entity = crate::entity::Entity::new(
                        self.clone(),
                        random_pos.to_f64(),
                        &EntityType::SKELETON_HORSE,
                    );
                    self.spawn_entity_non_save(Arc::new(entity));
                }
                let entity = crate::entity::Entity::new(
                    self.clone(),
                    random_pos.to_f64().add_raw(0.5, 0., 0.5),
                    &EntityType::LIGHTNING_BOLT,
                );
                self.spawn_entity_non_save(Arc::new(entity));
            }
        }

        if spawn_list.is_empty() {
            return;
        }
        // TODO this.level.canSpawnEntitiesInChunk(chunkPos)
        let entities = spawn_for_chunk(
            self,
            chunk_pos,
            chunk,
            spawn_state,
            spawn_list,
            is_thundering,
        );
        for entity in entities {
            self.spawn_entity_non_save(entity);
        }
    }
}
