use crate::block::entities::{BlockEntity, block_entity_from_nbt};
use dashmap::DashMap;
use pumpkin_data::chunk::Biome;
use pumpkin_data::item::{BedrockItem, BedrockItemVersion};
use pumpkin_protocol::bedrock::client::item_registry::{CItemRegistry, ItemData};
use pumpkin_protocol::bedrock::client::CBiomeDefinitionList;
use pumpkin_protocol::bedrock::network_item::{NetworkItemDescriptor, NetworkItemStackDescriptor};
use pumpkin_world::generation::proto_chunk::GenerationCache;
use rayon::prelude::*;
use std::sync::{Arc, RwLock, Weak};
use std::{
    collections::HashMap,
    sync::atomic::Ordering,
};
use tracing::{debug, error, info, warn};

mod active_chunks;
pub mod broadcast;
pub mod chunker;
pub mod damage;
pub mod explosion;
pub mod loot;
pub mod map;
pub mod particles;
pub mod portal;
pub mod raid;
pub mod random_sequences;
pub mod sounds;
pub mod stopwatches;
pub mod time;
pub mod villager_poi;

use crate::block::BlockEvent;
use crate::{
    block::registry::BlockRegistry,
    command::client_suggestions,
    entity::{Entity, EntityBase, player::Player, r#type::from_type},
    error::PumpkinError,
    net::{ClientPlatform, bedrock::BedrockClient, java::JavaClient},
    plugin::{
        player::{
            player_change_world::PlayerChangeWorldEvent, player_join::PlayerJoinEvent,
            player_respawn::PlayerRespawnEvent,
        },
    },
    server::Server,
};
use active_chunks::{ActiveChunkTracker, ActivePlayerArea};
use arc_swap::ArcSwap;
use border::Worldborder;
use bytes::BufMut;
pub use explosion::{
    BlockInteraction, DefaultExplosionDamageCalculator, Explosion, ExplosionDamageCalculator,
    ExplosionInteraction, SimpleExplosionDamageCalculator,
};
use pumpkin_config::BasicConfiguration;
use pumpkin_data::block_properties::{blocks_movement, is_air};
use pumpkin_data::block_rotation::{Mirror, Rotation};
use pumpkin_data::data_component_impl::EquipmentSlot;
use pumpkin_data::dimension::Dimension;
use pumpkin_data::fluid::FluidState;
use pumpkin_data::game_rules::{GameRule, GameRuleValue};
use pumpkin_data::noise_settings::NoiseSettings;
use pumpkin_data::{
    Block, BlockStateId,
    entity::EntityType,
    fluid::Fluid,
    particle::Particle,
    sound::Sound,
};
use pumpkin_data::{
    BlockDirection, BlockState, HorizontalFacingExt,
    block_properties::{ChestLikeProperties, ChestType},
    tag::Taggable,
    translation,
};
use pumpkin_inventory::crafting::recipe_provider::RecipeProvider;
use pumpkin_inventory::screen_handler::InventoryPlayer;
use pumpkin_nbt::compound::NbtCompound;
use pumpkin_protocol::bedrock::client::set_actor_data::{CSetActorData, PropertySyncData};
use pumpkin_protocol::bedrock::client::start_game::{CStartGame, ServerTelemetryData};
use pumpkin_protocol::java::client::play::{CExplosion, CRespawn, PlayerSpawnData};
use pumpkin_protocol::java::client::play::{CPlayerSpawnPosition, CRecipeBookAdd, CRecipeBookSettings};
use pumpkin_protocol::java::client::play::{CSetEntityMetadata, Metadata};
use pumpkin_protocol::{
    IdOr, SoundEvent,
    bedrock::{
        client::{
            add_player::CAddPlayer,
            common::BuildPlatform,
            creative_content::{
                CCreativeContent, CreativeCategory, CreativeGroupInfoPayload,
                CreativeItemEntryPayload,
            },
            player_list::{CPlayerList, PlayerListEntry},
            remove_actor::CRemoveActor,
            start_game::{Experiments, GamePublishSetting, LevelSettings},
            update_attributes::{AttributeData, CUpdateAttributes},
        },
        server::{
        },
    },
    codec::{var_int::VarInt, var_long::VarLong, var_uint::VarUInt, var_ulong::VarULong},
    java::{
        self,
        client::play::{
            CBlockEntityData, CGameEvent, CLogin, CPlayerInfoUpdate, CSetSelectedSlot,
            CSpawnEntity, GameEvent, InitChat, PlayerAction, PlayerInfoFlags,
        },
    },
};
use pumpkin_protocol::codec::item_stack_seralizer::ItemStackSerializer;
use pumpkin_protocol::java::client::play::{CParticle, CRemoveMobEffect, CSetEquipment, CUpdateMobEffect};
use pumpkin_util::resource_location::ResourceLocation;
use pumpkin_util::text::{TextComponent, color::NamedColor};
use pumpkin_util::version::JavaMinecraftVersion;
use pumpkin_util::{
    Difficulty,
    math::{boundingbox::BoundingBox, position::BlockPos, vector3::Vector3},
};
use pumpkin_util::math::vector2::Vector2;
use pumpkin_world::inventory::Clearable;
use pumpkin_world::world::{GetBlockError, WorldPortalExt};
use pumpkin_world::{
    CURRENT_BEDROCK_MC_VERSION, biome,
    chunk::io::Dirtiable,
    inventory::Inventory,
};
use pumpkin_world::{chunk::ChunkData, world::BlockAccessor};
use pumpkin_world::level::Level;
pub use pumpkin_world::{world::BlockFlags, world_info::LevelData};
use scoreboard::Scoreboard;
use time::LevelTime;

pub mod block_placer;
pub mod border;
pub mod bossbar;
pub mod custom_bossbar;
pub mod dragon_fight;
pub mod end_podium;
pub mod entity_tracker;
pub mod environment;
pub mod natural_spawner;
pub mod scoreboard;
pub mod block_updates;
pub mod dimension;
pub mod entities;
pub mod tick;
pub mod weather;

pub use dimension::calculate_celestial_angle;

pub use environment::EnvironmentAttributes;
pub use pumpkin_data::environment_attribute::{Activity, MoonPhase};

use crate::world::natural_spawner::SpawnState;
use pumpkin_config::lighting::LightingEngineConfig;
use pumpkin_world::chunk::ChunkHeightmapType::{self, MotionBlocking};
use uuid::Uuid;
use weather::Weather;

const MAX_LIGHT_LEVEL: u8 = 15;

fn bedrock_chest_block_actor(state_id: BlockStateId, position: BlockPos) -> Option<NbtCompound> {
    let (block, _) = BlockState::from_id_with_block(state_id);
    if !block.has_tag(&pumpkin_data::tag::Block::C_CHESTS_WOODEN)
        && !block.has_tag(&pumpkin_data::tag::Block::MINECRAFT_COPPER_CHESTS)
    {
        return None;
    }

    // Block actor tags describe the chest itself. Container contents are synchronized
    // through inventory packets and must not be exposed in chunk data.
    let mut nbt = NbtCompound::new();
    nbt.put_string("id", "Chest".to_string());
    nbt.put_int("x", position.0.x);
    nbt.put_int("y", position.0.y);
    nbt.put_int("z", position.0.z);
    nbt.put_bool("isMovable", true);

    let properties = ChestLikeProperties::from_state_id(state_id);
    if properties.r#type != ChestType::Single {
        let direction = if properties.r#type == ChestType::Left {
            properties.facing.rotate_clockwise()
        } else {
            properties.facing.rotate_counter_clockwise()
        };
        let pair = position.offset(direction.to_offset());
        nbt.put_int("pairx", pair.0.x);
        nbt.put_int("pairz", pair.0.z);
        if properties.r#type == ChestType::Right {
            nbt.put_bool("pairlead", true);
        }
    }

    Some(nbt)
}

use rustc_hash::{FxHashMap, FxHashSet};

impl PumpkinError for GetBlockError {
    fn is_kick(&self) -> bool {
        false
    }

    fn severity(&self) -> tracing::Level {
        tracing::Level::WARN
    }

    fn client_kick_reason(&self) -> Option<String> {
        None
    }
}

/// Represents a Minecraft world, containing entities, players, and the underlying level data.
///
/// Each dimension (Overworld, Nether, End) typically has its own `World`.
///
/// **Key Responsibilities:**
///
/// - Manages the `Level` instance for handling chunk-related operations.
/// - Stores and tracks active `Player` entities within the world.
/// - Provides a central hub for interacting with the world's entities and environment.
pub struct World {
    /// Represents the World's Unique Identifier
    pub uuid: Uuid,
    /// The underlying level, responsible for chunk management and terrain generation.
    pub level: Arc<Level>,
    pub level_info: Arc<ArcSwap<LevelData>>,
    /// A map of active players within the world, keyed by their unique UUID.
    pub players: ArcSwap<Vec<Arc<Player>>>,
    /// A map of active entities within the world, keyed by their unique UUID.
    /// This does not include players.
    pub entities: ArcSwap<Vec<Arc<dyn EntityBase>>>,
    /// The world's scoreboard, used for tracking scores, objectives, and display information.
    pub scoreboard: std::sync::Mutex<Scoreboard>,
    /// The world's worldborder, defining the playable area and controlling its expansion or contraction.
    pub worldborder: std::sync::Mutex<Worldborder>,
    /// The world's time, including counting ticks for weather, time cycles, and statistics.
    pub level_time: std::sync::Mutex<LevelTime>,
    /// The type of dimension the world is in.
    pub dimension: Dimension,
    pub sea_level: i32,
    pub min_y: i32,
    /// The world's weather, including rain and thunder levels.
    pub weather: std::sync::Mutex<Weather>,
    /// Block Behaviour
    pub block_registry: Arc<BlockRegistry>,
    pub server: Weak<Server>,
    synced_block_event_queue: std::sync::Mutex<Vec<BlockEvent>>,
    /// A map of unsent block changes, keyed by block position.
    unsent_block_changes: std::sync::Mutex<HashMap<BlockPos, BlockStateId>>,
    /// Persisted vanilla POI storage for portal and villager lookups.
    pub portal_poi: std::sync::Mutex<portal::PortalPoiStorage>,
    /// Villager job sites and their current owners.
    pub villager_poi: std::sync::Mutex<villager_poi::VillagerPoiStorage>,
    /// Active raids in this world.
    pub raids: std::sync::Mutex<raid::Raids>,
    /// End Dragon fight manager (only present in `THE_END` dimension).
    pub dragon_fight: Option<std::sync::Mutex<dragon_fight::DragonFight>>,
    pub spawn_state: ArcSwap<SpawnState>,
    pub active_chunks: RwLock<FxHashSet<Vector2<i32>>>,
    active_chunk_tracker: std::sync::Mutex<ActiveChunkTracker>,
    pub forced_chunks: std::sync::Mutex<FxHashSet<Vector2<i32>>>,
    /// Block entities indexed by chunk, so ticking only visits the currently
    /// active chunks instead of scanning every loaded block entity each tick.
    pub block_entities: DashMap<Vector2<i32>, FxHashMap<BlockPos, Arc<dyn BlockEntity>>>,
    pending_block_entity_migrations: crossbeam::queue::SegQueue<Vector2<i32>>,
    /// Persistent custom data for the world (matching Bukkit's `PersistentDataHolder`)
    pub custom_data: std::sync::Mutex<NbtCompound>,
    /// Persistent custom data for block entities at specific positions
    pub custom_block_entity_data: DashMap<BlockPos, NbtCompound>,
    /// Entity tracker responsible for tracking entity visibility and sending delta/status packets to watchers.
    pub entity_tracker: entity_tracker::EntityTracker,
}

#[derive(Clone, Copy)]
pub(crate) enum BlockBreakingProgress {
    Start { stage: i32, speed: f32 },
    Update { stage: i32, speed: Option<f32> },
    Stop,
}

impl PartialEq for World {
    fn eq(&self, other: &Self) -> bool {
        self.uuid == other.uuid
    }
}

impl Eq for World {}

impl World {
    pub async fn get_block_state_id_async(&self, position: &BlockPos) -> BlockStateId {
        if !self.is_in_build_limit(*position) {
            return Block::AIR.default_state.id;
        }

        let (chunk_coordinate, relative) = position.chunk_and_chunk_relative_position();
        self.level
            .get_or_fetch_chunk(chunk_coordinate, |chunk| {
                chunk
                    .section
                    .get_block_absolute_y(relative.x as usize, relative.y, relative.z as usize)
                    .unwrap_or(Block::AIR.default_state.id)
            })
            .await
    }

    pub async fn get_block_state_async(&self, position: &BlockPos) -> &'static BlockState {
        let id = self.get_block_state_id_async(position).await;
        BlockState::from_id(id)
    }

    pub async fn get_heightmap_height_async(
        &self,
        height_map: ChunkHeightmapType,
        x: i32,
        z: i32,
    ) -> i32 {
        let chunk_pos = Vector2::new(x >> 4, z >> 4);
        self.level
            .get_or_fetch_chunk(chunk_pos, |chunk| {
                chunk
                    .heightmap
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .get(height_map, x, z, self.min_y)
            })
            .await
    }

    #[must_use]
    pub fn load(
        level: Arc<Level>,
        level_info: Arc<ArcSwap<LevelData>>,
        dimension: Dimension,
        block_registry: Arc<BlockRegistry>,
        server: Weak<Server>,
    ) -> Self {
        // TODO
        let generation_settings = NoiseSettings::from_dimension(&dimension);

        // Load portal POI from disk (PoiStorage::new automatically loads from disk if files exist)
        let portal_poi = portal::PortalPoiStorage::new(level.level_folder.poi_folder.clone());
        let dragon_fight = (dimension.minecraft_name == Dimension::THE_END.minecraft_name)
            .then(|| std::sync::Mutex::new(dragon_fight::DragonFight::new()));

        let custom_data_path = level
            .level_folder
            .root_folder
            .join("pumpkin_custom_data.nbt");
        let custom_data = if custom_data_path.exists()
            && let Ok(bytes) = std::fs::read(&custom_data_path)
            && let Ok(nbt) = pumpkin_nbt::Nbt::read_unnamed(
                &mut pumpkin_nbt::deserializer::NbtReadHelperJava::new(&mut std::io::Cursor::new(
                    bytes,
                )),
            ) {
            nbt.root_tag
        } else {
            NbtCompound::new()
        };

        Self {
            uuid: Uuid::new_v4(),
            level,
            level_info,
            players: ArcSwap::new(Arc::new(Vec::new())),
            entities: ArcSwap::new(Arc::new(Vec::new())),
            scoreboard: std::sync::Mutex::new(Scoreboard::default()),
            worldborder: std::sync::Mutex::new(Worldborder::new(
                0.0,
                0.0,
                5.999_996_8E7,
                0,
                5,
                300,
            )),
            level_time: std::sync::Mutex::new(LevelTime::new()),
            dimension,
            weather: std::sync::Mutex::new(Weather::new()),
            block_registry,
            sea_level: generation_settings.sea_level,
            min_y: i32::from(generation_settings.shape.min_y),
            synced_block_event_queue: std::sync::Mutex::new(Vec::new()),
            unsent_block_changes: std::sync::Mutex::new(HashMap::new()),
            portal_poi: std::sync::Mutex::new(portal_poi),
            villager_poi: std::sync::Mutex::new(villager_poi::VillagerPoiStorage::default()),
            raids: std::sync::Mutex::new(raid::Raids::default()),
            dragon_fight,
            spawn_state: ArcSwap::new(Arc::new(SpawnState::empty())),
            active_chunks: RwLock::new(FxHashSet::default()),
            active_chunk_tracker: std::sync::Mutex::new(ActiveChunkTracker::default()),
            forced_chunks: std::sync::Mutex::new(FxHashSet::default()),
            server,
            block_entities: DashMap::new(),
            pending_block_entity_migrations: crossbeam::queue::SegQueue::new(),
            custom_data: std::sync::Mutex::new(custom_data),
            custom_block_entity_data: DashMap::new(),
            entity_tracker: entity_tracker::EntityTracker::new(),
        }
    }

    pub fn update_active_chunks(&self) {
        let sim_dist = self.server.upgrade().map_or(10, |s| {
            s.advanced_config.networking.java.simulation_distance.get()
        }) as i32;
        let players = self.players.load();
        let forced_chunks = self
            .forced_chunks
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        let mut tracker = self
            .active_chunk_tracker
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut active_chunks = self
            .active_chunks
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut newly_active = Vec::new();
        let mut current_players = FxHashSet::default();

        for player in players.iter() {
            let id = player.gameprofile.id;
            current_players.insert(id);
            tracker.update_player(
                id,
                ActivePlayerArea {
                    center: player.get_entity().chunk_pos.load(),
                    simulation_distance: sim_dist,
                },
                &mut active_chunks,
                &mut newly_active,
            );
        }
        let removed_players: Vec<_> = tracker
            .players
            .keys()
            .filter(|id| !current_players.contains(id))
            .copied()
            .collect();
        for id in removed_players {
            tracker.remove_player(id, &mut active_chunks);
        }
        tracker.sync_forced_chunks(&forced_chunks, &mut active_chunks, &mut newly_active);

        for pos in newly_active {
            if self.level.is_chunk_loaded(&pos) && tracker.loaded_active_chunks.insert(pos) {
                self.migrate_pending_block_entities(pos);
            }
        }
        for change in self.level.loaded_chunk_changes() {
            match change {
                pumpkin_world::level::LoadedChunkChange::Loaded(pos) => {
                    if active_chunks.contains(&pos)
                        && self.level.is_chunk_loaded(&pos)
                        && tracker.loaded_active_chunks.insert(pos)
                    {
                        self.migrate_pending_block_entities(pos);
                    }
                }
                pumpkin_world::level::LoadedChunkChange::Unloaded(pos) => {
                    if !self.level.is_chunk_loaded(&pos) {
                        tracker.loaded_active_chunks.remove(&pos);
                    }
                }
            }
        }
        let mut pending_migrations = FxHashSet::default();
        while let Some(pos) = self.pending_block_entity_migrations.pop() {
            pending_migrations.insert(pos);
        }
        for pos in pending_migrations {
            if active_chunks.contains(&pos) && self.level.is_chunk_loaded(&pos) {
                self.migrate_pending_block_entities(pos);
            }
        }
        let spawnable_chunks = tracker.loaded_active_chunks.len() as i32;
        drop(active_chunks);
        drop(tracker);

        self.spawn_state.store(Arc::new(SpawnState::new(
            spawnable_chunks,
            &self.entities,
            self,
        )));
    }

    /// Get the world folder name (e.g., `world`, `world_nether`, `world_the_end`).
    /// Falls back to "world" if the name cannot be determined.
    pub fn get_world_name(&self) -> &str {
        self.level
            .level_folder
            .root_folder
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("world")
    }

    /// Returns the configured shared world spawn block position and rotation.
    #[must_use]
    pub fn get_spawn_location(&self) -> (BlockPos, f32, f32) {
        let level_info = self.level_info.load();
        (
            BlockPos::new(level_info.spawn_x, level_info.spawn_y, level_info.spawn_z),
            level_info.spawn_yaw,
            level_info.spawn_pitch,
        )
    }

    pub async fn shutdown(&self) {
        for entity in self.entities.load().iter() {
            self.save_entity(entity).await;
        }

        let chunks: Vec<Vector2<i32>> = self
            .block_entities
            .iter()
            .map(|chunk_block_entities| *chunk_block_entities.key())
            .collect();
        for chunk_pos in chunks {
            self.save_block_entities(chunk_pos);
        }

        // Save portal POI to disk
        let save_result = self
            .portal_poi
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .save_all();
        if let Err(e) = save_result {
            error!("Failed to save portal POI: {e}");
        }

        self.level.shutdown().await;
    }

    /// Serializes a live entity into its current chunk's entity data. The live
    /// entity list is the source of truth while a chunk is loaded (its saved NBT
    /// is consumed on load), so this simply appends the entity to the chunk it is
    /// currently in; the chunk is rewritten from scratch every unload cycle, so
    /// there is nothing stale to deduplicate.
    async fn save_entity(&self, entity: &Arc<dyn EntityBase>) {
        let base_entity = entity.get_entity();
        if base_entity.is_removed() {
            return;
        }
        let current_chunk = base_entity.block_pos.load().chunk_position();
        let mut nbt = NbtCompound::new();
        entity.write_nbt(&mut nbt);
        let chunk = self.level.get_entity_chunk(current_chunk).await;
        chunk
            .data
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(nbt);
        chunk.mark_dirty(true);
    }

    /// Serializes the live block entities of a chunk back into that chunk's block
    /// entity data. The live map is the source of truth while a chunk is loaded -
    /// `get_block_entity` takes the saved NBT out of the chunk when it wakes an
    /// entity up - so this has to run before the chunk is dropped, or everything
    /// the entity did since it was loaded is lost.
    fn save_block_entities(&self, chunk_pos: Vector2<i32>) {
        let Some(block_entities) = self
            .block_entities
            .get(&chunk_pos)
            .map(|chunk_block_entities| chunk_block_entities.values().cloned().collect::<Vec<_>>())
        else {
            return;
        };

        for block_entity in block_entities {
            let mut nbt = NbtCompound::new();
            block_entity.write_internal(&mut nbt);
            if let Some(custom_data) = self
                .custom_block_entity_data
                .get(&block_entity.get_position())
                && !custom_data.is_empty()
            {
                nbt.put_compound("PumpkinCustomData", custom_data.clone());
            }
            self.add_block_entity_nbt(block_entity.get_position(), &nbt);
        }
    }

    pub fn set_difficulty(&self, difficulty: Difficulty) {
        let current_info = self.level_info.load();
        let mut new_info = (**current_info).clone();
        new_info.difficulty = difficulty;
        self.level_info.store(Arc::new(new_info));
    }

    pub fn get_game_rule(&self, rule: &GameRule) -> GameRuleValue<i64, bool> {
        let level_info = self.level_info.load();
        match level_info.game_rules.get(rule) {
            GameRuleValue::Int(v) => GameRuleValue::Int(*v),
            GameRuleValue::Bool(v) => GameRuleValue::Bool(*v),
        }
    }

    pub fn set_game_rule(&self, rule: &GameRule, value: GameRuleValue<i64, bool>) {
        let current_info = self.level_info.load();
        let mut new_info = (**current_info).clone();
        match (new_info.game_rules.get_mut(rule), value) {
            (GameRuleValue::Int(target), GameRuleValue::Int(val)) => {
                *target = val;
            }
            (GameRuleValue::Bool(target), GameRuleValue::Bool(val)) => {
                *target = val;
            }
            _ => {}
        }
        self.level_info.store(Arc::new(new_info));
    }

    pub fn check_fluid_collision(&self, bounding_box: BoundingBox) -> bool {
        let min = bounding_box.min_block_pos();

        let max = bounding_box.max_block_pos();

        for x in min.0.x..=max.0.x {
            for y in min.0.y..=max.0.y {
                for z in min.0.z..=max.0.z {
                    let pos = BlockPos::new(x, y, z);

                    let (fluid, state) = self.get_fluid_and_fluid_state(&pos);

                    if fluid.id != Fluid::EMPTY.id {
                        let height = f64::from(state.height);

                        if height >= bounding_box.min.y {
                            return true;
                        }
                    }
                }
            }
        }

        false
    }

    pub fn contains_any_liquid(&self, bounding_box: BoundingBox) -> bool {
        let min_x = bounding_box.min.x.floor() as i32;
        let max_x = bounding_box.max.x.ceil() as i32;
        let min_y = bounding_box.min.y.floor() as i32;
        let max_y = bounding_box.max.y.ceil() as i32;
        let min_z = bounding_box.min.z.floor() as i32;
        let max_z = bounding_box.max.z.ceil() as i32;

        for x in min_x..max_x {
            for y in min_y..max_y {
                for z in min_z..max_z {
                    let pos = BlockPos::new(x, y, z);
                    if self.get_fluid_and_fluid_state(&pos).0.id != Fluid::EMPTY.id {
                        return true;
                    }
                }
            }
        }

        false
    }

    // FlowingFluid.getFlow()
    pub fn get_fluid_velocity(
        &self,
        pos0: BlockPos,
        fluid0: &Fluid,
        state0: &FluidState,
    ) -> Vector3<f64> {
        let mut velo = Vector3::default();

        for dir in BlockDirection::horizontal() {
            let offset = dir.to_offset();
            let pos = pos0.offset(offset);

            let (neighbor_fluid, neighbor_state) = self.get_fluid_and_fluid_state(&pos);

            if neighbor_fluid.matches_type(fluid0) {
                let mut neighbor_height = neighbor_state.height;
                let mut amplitude = 0.0;

                if neighbor_height == 0.0 {
                    let state_id = self.get_block_state_id(&pos);
                    let block_id = state_id.to_block_id();
                    let block_state = state_id.to_state();

                    let blocks_movement = blocks_movement(block_state, block_id);

                    if !blocks_movement {
                        let down_pos = pos.down();
                        let (down_fluid, down_state) = self.get_fluid_and_fluid_state(&down_pos);

                        if down_fluid.matches_type(fluid0) {
                            neighbor_height = down_state.height;
                            if neighbor_height > 0.0 {
                                amplitude = f64::from(state0.height)
                                    - (f64::from(neighbor_height) - 0.888_888_9);
                            }
                        }
                    }
                } else if neighbor_height > 0.0 {
                    amplitude = f64::from(state0.height) - f64::from(neighbor_height);
                }

                if amplitude != 0.0 {
                    velo.x += f64::from(offset.x) * amplitude;
                    velo.z += f64::from(offset.z) * amplitude;
                }
            }
        }

        if state0.falling {
            for dir in BlockDirection::horizontal() {
                let pos = pos0.offset(dir.to_offset());

                if self.is_solid_face(fluid0.id, pos, dir.to_block_direction())
                    || self.is_solid_face(fluid0.id, pos.up(), dir.to_block_direction())
                {
                    if velo.length_squared() != 0.0 {
                        velo = velo.normalize();
                    }

                    velo.y -= 6.0;
                    break;
                }
            }
        }

        if velo.length_squared() == 0.0 {
            velo
        } else {
            velo.normalize()
        }
    }

    // FlowingFluid.isSolidFace()
    fn is_solid_face(&self, fluid0_id: u16, pos: BlockPos, direction: BlockDirection) -> bool {
        let id = self.get_block_state_id(&pos);

        let fluid = Fluid::from_state_id(id).unwrap_or(&Fluid::EMPTY);

        if Fluid::same_fluid_type(fluid.id, fluid0_id) {
            return false;
        }

        if direction == BlockDirection::Up {
            return true;
        }

        let block = Block::from_state_id(id);
        let state = BlockState::from_id(id);

        // Doesn't count blue ice or packed ice

        if block == &Block::ICE || block == &Block::FROSTED_ICE {
            return false;
        }

        state.is_side_solid(direction)
    }

    pub fn check_outline<F>(
        bounding_box: &BoundingBox,
        pos: BlockPos,
        state: &BlockState,
        use_outline_shape: bool,
        mut using_outline_shape: F,
    ) -> bool
    where
        F: FnMut(&BoundingBox),
    {
        if state.outline_shapes.is_empty() {
            // Apparently we need this for air and moving pistons

            return true;
        }

        let mut inside = false;
        'shapes: for shape in state.get_block_outline_shapes_at(&pos) {
            let outline_shape = shape.at_pos(pos);

            if outline_shape.intersects(bounding_box) {
                inside = true;

                if !use_outline_shape {
                    break 'shapes;
                }

                using_outline_shape(&outline_shape);
            }
        }

        inside
    }

    pub fn check_collision<F>(
        bounding_box: &BoundingBox,
        pos: BlockPos,
        state: &BlockState,
        use_collision_shape: bool,
        mut on_collision: F,
    ) -> bool
    where
        F: FnMut(&BoundingBox),
    {
        if state.is_air() || !state.is_solid() {
            return false;
        }

        let mut shapes = state
            .get_block_collision_shapes_at(&pos)
            .map(|shape| shape.at_pos(pos));

        if use_collision_shape {
            let mut collided = false;
            for collision_shape in shapes {
                if collision_shape.intersects(bounding_box) {
                    collided = true;
                    // Convert to BB and trigger the callback
                    on_collision(&collision_shape);
                }
            }
            collided
        } else {
            shapes.any(|s| s.intersects(bounding_box))
        }
    }

    // For adjusting movement
    pub fn get_block_collisions(
        &self,
        bounding_box: BoundingBox,
        entity: &dyn EntityBase,
    ) -> (Vec<BoundingBox>, Vec<(usize, BlockPos)>) {
        let mut collisions = Vec::new();

        let mut positions = Vec::new();

        let min = BlockPos::floored_v(bounding_box.min.add_raw(0.0, -0.50001, 0.0));
        let max = bounding_box.max_block_pos();
        let pos_iter = BlockPos::iterate(min, max);

        for pos in pos_iter {
            let state = self.get_block_state(&pos);

            if state.is_air() {
                continue;
            }

            let block = Block::from_state_id(state.id);
            let mut collided = false;

            if block == &Block::POWDER_SNOW {
                if let Some(shape) =
                    crate::block::blocks::powder_snow::collision_shape_for_entity(entity, &pos)
                {
                    let shape = shape.at_pos(pos);
                    if shape.intersects(&bounding_box) {
                        collided = true;
                        collisions.push(shape);
                    }
                }
            } else {
                for shape in state.get_block_collision_shapes_at(&pos) {
                    let shape = shape.at_pos(pos);
                    if shape.intersects(&bounding_box) {
                        collided = true;
                        collisions.push(shape);
                    }
                }
            }

            if collided {
                positions.push((collisions.len(), pos));
            }
        }

        (collisions, positions)
    }

    pub fn is_space_empty(&self, bounding_box: BoundingBox) -> bool {
        let min = bounding_box.min_block_pos();
        let max = bounding_box.max_block_pos();

        for pos in BlockPos::iterate(min, max) {
            let state = self.get_block_state(&pos);
            let collided = Self::check_collision(&bounding_box, pos, state, false, |_| ());

            if collided {
                return false;
            }
        }
        true
    }

    /// Vanilla's `BlockView.getDismountHeight()`.
    /// Returns the Y surface height for dismounting at the given block position,
    /// or `f64::NEG_INFINITY` if no valid surface exists.
    pub fn get_dismount_height(&self, pos: &BlockPos) -> f64 {
        let state = self.get_block_state(pos);
        let max_y = state
            .get_block_collision_shapes_at(pos)
            .map(|s| s.max.y)
            .fold(f64::NEG_INFINITY, f64::max);
        if max_y != f64::NEG_INFINITY {
            return max_y;
        }
        // No collision at pos — check block below
        let below = BlockPos(Vector3::new(pos.0.x, pos.0.y - 1, pos.0.z));
        let below_state = self.get_block_state(&below);
        let below_max_y = below_state
            .get_block_collision_shapes_at(&below)
            .map(|s| s.max.y)
            .fold(f64::NEG_INFINITY, f64::max);
        if below_max_y >= 1.0 {
            below_max_y - 1.0
        } else {
            f64::NEG_INFINITY
        }
    }

    pub fn get_world_age(&self) -> i64 {
        self.level_time
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .world_age
    }

    pub fn get_time_of_day(&self) -> i64 {
        self.level_time
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .time_of_day
    }

    pub fn set_time_of_day(&self, time: i64) {
        let level_time = {
            let mut guard = self
                .level_time
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            guard.set_time(time);
            guard.clone()
        };
        level_time.send_time(self);
    }

    pub fn is_raining(&self) -> bool {
        self.weather
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .raining
    }

    pub fn is_raining_at(&self, pos: &BlockPos) -> bool {
        if !self.is_raining() {
            return false;
        }
        if self.get_heightmap_height(MotionBlocking, pos.0.x, pos.0.z) + 1 > pos.0.y {
            return false;
        }
        self.can_see_sky(pos)
            && self
                .get_biome(pos)
                .weather
                .is_rain_at(pos.0.x, pos.0.y, pos.0.z, self.sea_level)
    }

    pub fn set_raining(&self, raining: bool) {
        if let Some(server) = self.server.upgrade() {
            let world_arc = server.get_world_from_dimension(&self.dimension);
            let mut event =
                crate::plugin::api::events::world::weather_change::WeatherChangeEvent::new(
                    world_arc, raining,
                );
            server.plugin_manager.fire_blocking(&server, &mut event);
            if event.cancelled {
                return;
            }
        }
        let mut weather = self
            .weather
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if weather.raining != raining {
            let thunder = weather.thundering;
            weather.set_weather_parameters(self, 0, 0, raining, thunder);
        }
    }

    pub fn is_thundering(&self) -> bool {
        self.weather
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .thundering
    }

    pub fn set_thundering(&self, thundering: bool) {
        if let Some(server) = self.server.upgrade() {
            let world_arc = server.get_world_from_dimension(&self.dimension);
            let mut event =
                crate::plugin::api::events::world::weather_change::ThunderChangeEvent::new(
                    world_arc, thundering,
                );
            server.plugin_manager.fire_blocking(&server, &mut event);
            if event.cancelled {
                return;
            }
        }
        let mut weather = self
            .weather
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if weather.thundering != thundering {
            let raining = weather.raining;
            weather.set_weather_parameters(self, 0, 0, raining, thundering);
        }
    }

    /// Gets the y position of the first non air block from the top down
    pub fn get_top_block(&self, position: Vector2<i32>) -> i32 {
        let chunk_pos = Vector2::new(position.x >> 4, position.y >> 4);
        let relative_x = (position.x & 15) as usize;
        let relative_z = (position.y & 15) as usize;

        self.level
            .read_chunk_sync(&chunk_pos, |chunk| {
                let height = chunk
                    .heightmap
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .get(
                        ChunkHeightmapType::WorldSurface,
                        position.x,
                        position.y,
                        self.dimension.min_y,
                    );

                if height >= self.dimension.min_y {
                    return height;
                }

                for y in (self.dimension.min_y..self.dimension.min_y + self.dimension.height).rev()
                {
                    if let Some(block_id) = chunk
                        .section
                        .get_block_absolute_y(relative_x, y, relative_z)
                        && !is_air(block_id)
                    {
                        return y;
                    }
                }
                self.dimension.min_y
            })
            .unwrap_or(self.dimension.min_y)
    }

    pub fn get_heightmap_height(&self, height_map: ChunkHeightmapType, x: i32, z: i32) -> i32 {
        let chunk_pos = Vector2::new(x >> 4, z >> 4);
        self.level
            .read_chunk_sync(&chunk_pos, |chunk| {
                chunk
                    .heightmap
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .get(height_map, x, z, self.min_y)
            })
            .unwrap_or(self.min_y)
    }

    #[allow(clippy::too_many_lines)]
    pub async fn spawn_bedrock_player(
        &self,
        base_config: &BasicConfiguration,
        player: Arc<Player>,
        server: &Arc<Server>,
    ) {
        static CREATIVE_CONTENT: std::sync::OnceLock<(
            Vec<CreativeGroupInfoPayload>,
            Vec<CreativeItemEntryPayload>,
        )> = std::sync::OnceLock::new();

        static BEDROCK_CRAFTING_DATA: std::sync::OnceLock<
            Vec<pumpkin_protocol::bedrock::client::BedrockRecipe>,
        > = std::sync::OnceLock::new();

        let level_info = server.level_info.load();
        let (rain_level, lightning_level) = {
            let weather = self
                .weather
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            (weather.rain_level, weather.thunder_level)
        };
        let runtime_id = player.entity_id() as u64;
        let (position, yaw, pitch) = if player.has_played_before.load(Ordering::Relaxed) {
            let position = player.position();
            let yaw = player.get_entity().yaw.load(); //info.spawn_angle;
            let pitch = player.get_entity().pitch.load();

            (position, yaw, pitch)
        } else {
            let spawn_position = Vector2::new(level_info.spawn_x, level_info.spawn_z);
            let chunk_pos = Vector2::new(level_info.spawn_x >> 4, level_info.spawn_z >> 4);
            self.level.get_or_fetch_chunk(chunk_pos, |_| ()).await;
            let top = self.get_top_block(spawn_position);
            let pos_y = if top > self.dimension.min_y {
                top + 1
            } else {
                level_info.spawn_y
            };

            let position = Vector3::new(
                f64::from(level_info.spawn_x) + 0.5,
                f64::from(pos_y),
                f64::from(level_info.spawn_z) + 0.5,
            );
            (position, level_info.spawn_yaw, level_info.spawn_pitch)
        };

        // Keep the server-side transform aligned with the StartGame position. In
        // particular, this ensures an early disconnect persists the real spawn.
        player.living_entity.entity.set_pos(position);
        player.living_entity.entity.set_rotation(yaw, pitch);
        player.living_entity.entity.last_pos.store(position);

        // Todo make the data less spread
        let level_settings = LevelSettings {
            seed: self.level.seed.0,
            spawn_biome_type: 0,
            custom_biome_name: String::new(),
            dimension: VarInt(0),
            generator_type: VarInt(1),
            world_gamemode: server
                .defaultgamemode
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .gamemode
                .into(),
            hardcore: base_config.hardcore,
            difficulty: VarInt(level_info.difficulty as i32),
            spawn_position: BlockPos::new(
                level_info.spawn_x,
                level_info.spawn_y,
                level_info.spawn_z,
            ),
            has_achievements_disabled: false,
            editor_world_type: VarInt(0),
            is_created_in_editor: false,
            is_exported_from_editor: false,
            day_cycle_stop_time: VarInt(-1),
            education_edition_offer: VarUInt(0),
            has_education_features_enabled: false,
            education_product_id: String::new(),
            rain_level,
            lightning_level,
            has_confirmed_platform_locked_content: false,
            was_multiplayer_intended: true,
            was_lan_broadcasting_intended: true,
            xbox_live_broadcast_setting: GamePublishSetting::Public,
            platform_broadcast_setting: GamePublishSetting::Public,
            commands_enabled: level_info.allow_commands,
            is_texture_packs_required: false,
            rule_data: Vec::new(),
            experiments: Experiments {
                toggles: Vec::new(),
                experiments_ever_toggled: false,
            },
            bonus_chest: false,
            has_start_with_map_enabled: false,
            // TODO Bedrock permission level are different
            permission_level: 2,
            server_simulation_distance: server
                .advanced_config
                .networking
                .bedrock
                .simulation_distance
                .get()
                .into(),
            has_locked_behavior_pack: false,
            has_locked_resource_pack: false,
            is_from_locked_world_template: false,
            is_using_msa_gamertags_only: false,
            is_from_world_template: false,
            is_world_template_option_locked: false,
            is_only_spawning_v1_villagers: false,
            is_disabling_personas: false,
            is_disabling_custom_skins: false,
            emote_chat_muted: false,
            game_version: CURRENT_BEDROCK_MC_VERSION.into(),
            limited_world_width: 0,
            limited_world_height: 0,
            new_nether: true,
            edu_shared_uri_button_name: String::new(),
            edu_shared_uri_link_uri: String::new(),
            override_force_experimental_gameplay_has_value: false,
            chat_restriction_level: 0,
            disable_player_interactions: false,
            server_editor_connection_policy: VarInt(0),
            allow_anonymous_block_drops_in_editor_worlds: false,
        };
        drop(level_info);

        let Some(client) = player.client.bedrock() else {
            return;
        };

        let start_game = CStartGame {
            entity_id: VarLong(runtime_id as _),
            runtime_entity_id: VarULong(runtime_id),
            player_gamemode: player.gamemode.load().into(),
            // Bedrock represents the local player at eye height; Pumpkin stores feet position.
            position: Vector3::new(
                position.x as f32,
                position.y as f32 + player.get_entity().entity_type.eye_height,
                position.z as f32,
            ),
            pitch,
            yaw,
            level_settings,
            level_id: String::new(),
            level_name: "Potato world".to_string(),
            premium_world_template_id: String::new(),
            is_trial: false,
            rewind_history_size: VarInt(0),
            server_authoritative_block_breaking: true,
            current_level_time: self.get_world_age() as _,
            enchantment_seed: VarInt(0),
            block_properties_size: VarUInt(0),
            // TODO Make this unique
            multiplayer_correlation_id: Uuid::default().to_string(),
            enable_itemstack_net_manager: true,
            server_version: "Potato Rust Server".to_string(),
            compound_id: 10,
            compound_len: VarUInt(0),
            compound_end: 0,
            block_registry_checksum: 0,
            world_template_id: Uuid::nil(),
            enable_clientside_generation: false,
            blocknetwork_ids_are_hashed: false,
            server_auth_sounds: true,
            server_join_information: None,
            telemetry: ServerTelemetryData {
                server_id: String::new(),
                scenario_id: String::new(),
                world_id: String::new(),
                owner_id: String::new(),
            },
        };
        if let Ok(data) = client.serialize_packet(&start_game) {
            client.send_game_packet(data).await;
        }

        if let Ok(data) = client.serialize_packet(&CBiomeDefinitionList) {
            client.send_game_packet(data).await;
        }

        let item_registry = CItemRegistry {
            items: BedrockItem::ALL_BEDROCK_ITEMS
                .iter()
                .map(|b| ItemData {
                    item_name: b.registry_key.into(),
                    item_id: b.id,
                    is_component_based: b.component_based,
                    item_version: VarInt::from(match b.version {
                        BedrockItemVersion::Legacy => 0,
                        BedrockItemVersion::DataDriven => 1,
                        BedrockItemVersion::None => 2,
                    }),
                    component_data: b.definition_components.into(),
                })
                .collect::<Vec<_>>(),
        };
        if let Ok(data) = client.serialize_packet(&item_registry) {
            client.send_game_packet(data).await;
        }

        let (groups, entries) = CREATIVE_CONTENT.get_or_init(|| {
            let groups = pumpkin_data::bedrock_creative::CREATIVE_GROUPS
                .iter()
                .map(|g| {
                    let creative_category = match g.category {
                        1 => CreativeCategory::Construction,
                        2 => CreativeCategory::Nature,
                        3 => CreativeCategory::Equipment,
                        4 => CreativeCategory::Items,
                        5 => CreativeCategory::ItemCommandOnly,
                        _ => CreativeCategory::Undefined,
                    };
                    let icon_item = if g.icon_item_id != 0 {
                        NetworkItemDescriptor {
                            id: VarInt::from(g.icon_item_id),
                            stack_size: 1,
                            aux_value: VarUInt(g.icon_item_aux_value),
                            block_runtime_id: VarInt(0),
                            nbt_data: pumpkin_nbt::Nbt::default(),
                            place_on_blocks: Vec::new(),
                            destroy_blocks: Vec::new(),
                            shield_blocking_tick: 0,
                        }
                    } else {
                        NetworkItemDescriptor::default()
                    };

                    CreativeGroupInfoPayload {
                        creative_category,
                        name: g.name.to_string(),
                        group_icon_item: icon_item,
                    }
                })
                .collect::<Vec<_>>();

            let entries = pumpkin_data::bedrock_creative::CREATIVE_ENTRIES
                .iter()
                .enumerate()
                .map(|(i, e)| CreativeItemEntryPayload {
                    id: VarUInt((i + 1) as u32),
                    item: NetworkItemDescriptor {
                        id: VarInt::from(e.item_id),
                        stack_size: 1,
                        aux_value: VarUInt(e.item_aux_value),
                        block_runtime_id: VarInt(0),
                        nbt_data: pumpkin_nbt::Nbt::default(),
                        place_on_blocks: Vec::new(),
                        destroy_blocks: Vec::new(),
                        shield_blocking_tick: 0,
                    },
                    group_index: VarUInt(e.group_index),
                })
                .collect::<Vec<_>>();

            (groups, entries)
        });
        let creative_content = CCreativeContent { groups, entries };
        if let Ok(data) = client.serialize_packet(&creative_content) {
            client.send_game_packet(data).await;
        }

        let bedrock_recipes = BEDROCK_CRAFTING_DATA.get_or_init(|| {
            use pumpkin_data::item::{Item, JavaToBedrockItemMapping};
            use pumpkin_data::recipes::{CraftingRecipeTypes, RecipeIngredientTypes};
            use pumpkin_protocol::bedrock::client::{
                BedrockRecipe, BedrockShapedRecipe, BedrockShapelessRecipe, ItemDescriptorCount,
                RecipeUnlockRequirement,
            };
            use pumpkin_protocol::bedrock::network_item::NetworkItemDescriptor;
            use pumpkin_protocol::codec::{var_int::VarInt, var_uint::VarUInt};

            let mut mapped_recipes = Vec::new();
            let mut network_id_counter = 1u32;

            for recipe in pumpkin_data::recipes::RECIPES_CRAFTING {
                let map_ingredient = |ing: &RecipeIngredientTypes| -> ItemDescriptorCount {
                    let item_key = match ing {
                        RecipeIngredientTypes::Simple(name) => Some(*name),
                        RecipeIngredientTypes::Tagged(tag) => {
                            let tag_name = tag.strip_prefix('#').unwrap_or(tag);
                            pumpkin_data::tag::get_tag_ids(
                                pumpkin_data::tag::RegistryKey::Item,
                                tag_name,
                            )
                            .and_then(|ids| {
                                ids.first().and_then(|&first_id| {
                                    Item::from_id(first_id).map(|item| item.registry_key)
                                })
                            })
                        }
                        RecipeIngredientTypes::OneOf(names) => names.first().copied(),
                    };

                    if let Some(key) = item_key {
                        let registry_key = key.strip_prefix("minecraft:").unwrap_or(key);
                        if let Some(item) = Item::from_registry_key(registry_key)
                            && let Some(mapping) =
                                JavaToBedrockItemMapping::from_java_item_id(item.id)
                        {
                            return ItemDescriptorCount {
                                item_identifier: mapping.bedrock_item.registry_key.to_string(),
                                metadata_value: mapping.bedrock_data as i32,
                                count: 1,
                            };
                        }
                    }

                    ItemDescriptorCount {
                        item_identifier: String::new(),
                        metadata_value: 0,
                        count: 0,
                    }
                };

                match recipe {
                    CraftingRecipeTypes::CraftingShaped {
                        category: _,
                        group: _,
                        show_notification: _,
                        key,
                        pattern,
                        result,
                    } => {
                        let height = pattern.len() as i32;
                        let width = pattern.iter().map(|s| s.len()).max().unwrap_or(0) as i32;

                        let mut input = Vec::new();
                        for r in 0..height {
                            let pattern_row = pattern[r as usize];
                            for c in 0..width {
                                let ch = pattern_row.chars().nth(c as usize).unwrap_or(' ');
                                if ch == ' ' {
                                    input.push(ItemDescriptorCount {
                                        item_identifier: String::new(),
                                        metadata_value: 0,
                                        count: 0,
                                    });
                                } else {
                                    let mut ingredient = None;
                                    for &(key_ch, ref ing) in *key {
                                        if key_ch == ch {
                                            ingredient = Some(ing);
                                            break;
                                        }
                                    }
                                    if let Some(ing) = ingredient {
                                        input.push(map_ingredient(ing));
                                    } else {
                                        input.push(ItemDescriptorCount {
                                            item_identifier: String::new(),
                                            metadata_value: 0,
                                            count: 0,
                                        });
                                    }
                                }
                            }
                        }

                        let output_item = Item::from_registry_key(result.id);
                        if let Some(item) = output_item
                            && let Some(mapping) =
                                JavaToBedrockItemMapping::from_java_item_id(item.id)
                        {
                            let output_descriptor = NetworkItemDescriptor {
                                id: VarInt::from(mapping.bedrock_item.id),
                                stack_size: result.count as u16,
                                aux_value: VarUInt(mapping.bedrock_data),
                                block_runtime_id: VarInt::from(mapping.bedrock_block_state),
                                nbt_data: pumpkin_nbt::Nbt::default(),
                                place_on_blocks: Vec::new(),
                                destroy_blocks: Vec::new(),
                                shield_blocking_tick: 0,
                            };

                            mapped_recipes.push(BedrockRecipe::Shaped(BedrockShapedRecipe {
                                recipe_id: format!("pumpkin:recipe_{network_id_counter}"),
                                width: VarInt(width),
                                height: VarInt(height),
                                input,
                                output: vec![output_descriptor],
                                uuid: Uuid::nil(),
                                block: "crafting_table".to_string(),
                                priority: VarInt(1),
                                assume_symmetry: true,
                                unlock_requirement: RecipeUnlockRequirement { context: 1 },
                                recipe_network_id: VarUInt(network_id_counter),
                            }));
                            network_id_counter += 1;
                        }
                    }
                    CraftingRecipeTypes::CraftingShapeless {
                        category: _,
                        group: _,
                        ingredients,
                        result,
                    } => {
                        let input = ingredients.iter().map(map_ingredient).collect::<Vec<_>>();

                        let output_item = Item::from_registry_key(result.id);
                        if let Some(item) = output_item
                            && let Some(mapping) =
                                JavaToBedrockItemMapping::from_java_item_id(item.id)
                        {
                            let output_descriptor = NetworkItemDescriptor {
                                id: VarInt::from(mapping.bedrock_item.id),
                                stack_size: result.count as u16,
                                aux_value: VarUInt(mapping.bedrock_data),
                                block_runtime_id: VarInt::from(mapping.bedrock_block_state),
                                nbt_data: pumpkin_nbt::Nbt::default(),
                                place_on_blocks: Vec::new(),
                                destroy_blocks: Vec::new(),
                                shield_blocking_tick: 0,
                            };

                            mapped_recipes.push(BedrockRecipe::Shapeless(BedrockShapelessRecipe {
                                recipe_id: format!("pumpkin:recipe_{network_id_counter}"),
                                input,
                                output: vec![output_descriptor],
                                uuid: Uuid::nil(),
                                block: "crafting_table".to_string(),
                                priority: VarInt(1),
                                unlock_requirement: RecipeUnlockRequirement { context: 1 },
                                recipe_network_id: VarUInt(network_id_counter),
                            }));
                            network_id_counter += 1;
                        }
                    }
                    _ => {}
                }
            }
            mapped_recipes
        });

        let crafting_data = pumpkin_protocol::bedrock::client::CCraftingData {
            recipes: bedrock_recipes.clone(),
            clean_recipes: false,
        };
        if let Ok(data) = client.serialize_packet(&crafting_data) {
            client.send_game_packet(data).await;
        }

        player.on_screen_handler_opened(&player.player_screen_handler);

        {
            let mut abilities = player
                .abilities
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            abilities.set_for_gamemode(player.gamemode.load());
        };

        let entity = &player.get_entity();
        let metadata = entity.bedrock_metadata();

        let actor_data = CSetActorData {
            target_runtime_id: VarULong(runtime_id),
            actor_data: metadata,
            synced_properties: PropertySyncData {
                int_entries_list: HashMap::new(),
                float_entries_list: HashMap::new(),
            },
            tick: VarULong(0),
        };
        if let Ok(data) = client.serialize_packet(&actor_data) {
            client.send_game_packet(data).await;
        }
        player.send_abilities_update();

        {
            let command_dispatcher = server.command_dispatcher.load();
            client_suggestions::send_bedrock_commands_packet(&player, server, &command_dispatcher);
        };

        client
            .enqueue_client_packet(&CUpdateAttributes {
                target_runtime_id: VarULong(runtime_id),
                attribute_list: vec![
                    AttributeData {
                        min_value: 0.0,
                        max_value: 3.402_823_5E38,
                        current_value: 0.1,
                        default_min_value: 0.0,
                        default_max_value: 3.402_823_5E38,
                        default_value: 0.1,
                        name: "minecraft:movement".to_string(),
                        modifiers: Vec::new(),
                    },
                    AttributeData {
                        min_value: 0.0,
                        max_value: 3.402_823_5E38,
                        current_value: 0.02,
                        default_min_value: 0.0,
                        default_max_value: 3.402_823_5E38,
                        default_value: 0.02,
                        name: "minecraft:underwater_movement".to_string(),
                        modifiers: Vec::new(),
                    },
                    AttributeData {
                        min_value: 0.0,
                        max_value: 1.0,
                        current_value: 0.08,
                        default_min_value: 0.0,
                        default_max_value: 1.0,
                        default_value: 0.08,
                        name: "minecraft:gravity".to_string(),
                        modifiers: Vec::new(),
                    },
                    AttributeData {
                        min_value: 0.0,
                        max_value: 400.0,
                        current_value: 400.0,
                        default_min_value: 0.0,
                        default_max_value: 400.0,
                        default_value: 400.0,
                        name: "minecraft:air".to_string(),
                        modifiers: Vec::new(),
                    },
                    AttributeData {
                        min_value: 0.0,
                        max_value: 20.0,
                        current_value: player.living_entity.health.load(),
                        default_min_value: 0.0,
                        default_max_value: 20.0,
                        default_value: 20.0,
                        name: "minecraft:health".to_string(),
                        modifiers: Vec::new(),
                    },
                    AttributeData {
                        min_value: 0.0,
                        max_value: 20.0,
                        current_value: player.hunger_manager.level.load().into(),
                        default_min_value: 0.0,
                        default_max_value: 20.0,
                        default_value: 20.0,
                        name: "minecraft:player.hunger".to_string(),
                        modifiers: Vec::new(),
                    },
                ],
                tick: VarULong(0),
            })
            .await;

        // --- MULTIPLAYER BROADCASTING ---

        let gameprofile = &player.gameprofile;
        let velocity = player.get_entity().velocity.load();

        // 1. Broadcast the new Bedrock player to everyone else (Java + Bedrock)
        let bedrock_player_list = CPlayerList {
            action: CPlayerList::ACTION_ADD,
            entries: vec![PlayerListEntry {
                uuid: gameprofile.id,
                entity_unique_id: VarLong(runtime_id as i64),
                username: gameprofile.name.clone(),
                xuid: String::new(),
                platform_chat_id: String::new(),
                build_platform: BuildPlatform::Unknown,
                skin: (**player.bedrock_skin.load()).clone(),
                is_teacher: false,
                is_host: false,
                is_sub_client: false,
                player_color: [0, 0, 0, 0],
            }],
        };

        let gamemode = player.gamemode.load();
        self.broadcast_packet_except_editioned(
            &[gameprofile.id],
            &CPlayerInfoUpdate::new(
                (PlayerInfoFlags::ADD_PLAYER
                    | PlayerInfoFlags::UPDATE_GAME_MODE
                    | PlayerInfoFlags::UPDATE_LISTED
                    | PlayerInfoFlags::UPDATE_LATENCY
                    | PlayerInfoFlags::UPDATE_LIST_PRIORITY
                    | PlayerInfoFlags::UPDATE_HAT)
                    .bits(),
                &[pumpkin_protocol::java::client::play::Player {
                    uuid: gameprofile.id,
                    actions: &[
                        PlayerAction::AddPlayer {
                            name: &gameprofile.name,
                            properties: &gameprofile.properties.load(),
                        },
                        PlayerAction::UpdateGameMode(VarInt(gamemode as i32)),
                        PlayerAction::UpdateListed(true),
                        PlayerAction::UpdateLatency(VarInt(0)),
                        PlayerAction::UpdateListOrder(VarInt(0)),
                        PlayerAction::UpdateHat(true),
                    ],
                }],
            ),
            &bedrock_player_list,
        );

        let bedrock_add_player = CAddPlayer {
            uuid: gameprofile.id,
            player_name: gameprofile.name.clone(),
            target_runtime_id: VarULong(runtime_id),
            platform_chat_id: String::new(),
            position: Vector3::new(position.x as f32, position.y as f32, position.z as f32),
            velocity: Vector3::new(velocity.x as f32, velocity.y as f32, velocity.z as f32),
            rotation: Vector2::new(pitch, yaw),
            y_head_rotation: yaw,
            carried_item: NetworkItemStackDescriptor::default(),
            player_game_type: player.gamemode.load().into(),
            entity_data: entity.bedrock_metadata(),
            synced_properties: PropertySyncData::default(),
            abilities_data: pumpkin_protocol::bedrock::client::SerializedAbilitiesData {
                target_player_raw_id: runtime_id as i64,
                player_permissions:
                    pumpkin_protocol::bedrock::client::PlayerPermissionLevel::Visitor,
                command_permissions: pumpkin_protocol::bedrock::client::CommandPermissionLevel::Any,
                layers: vec![
                    pumpkin_protocol::bedrock::client::SerializedAbilitiesDataSerializedLayer {
                        serialized_layer: 0,
                        abilities_set: 0,
                        ability_value: 0,
                        fly_speed: 0.05,
                        vertical_fly_speed: 0.05,
                        walk_speed: 0.1,
                    },
                ],
            },
            actor_links: Vec::new(),
            device_id: String::new(),
            build_platform: BuildPlatform::Unknown,
        };

        self.broadcast_packet_except_editioned(
            &[gameprofile.id],
            &CSpawnEntity::new(
                (runtime_id as i32).into(),
                gameprofile.id,
                i32::from(EntityType::PLAYER.id).into(),
                position,
                pitch,
                yaw,
                yaw,
                0.into(),
                velocity,
            ),
            &bedrock_add_player,
        );

        self.send_player_equipment(&player);

        // Broadcast metadata to Java players so they can correctly interact with the new player
        let skin_parts = player.config.load().skin_parts;

        self.broadcast_skin_parts(
            &[gameprofile.id],
            runtime_id as i32,
            skin_parts,
            &actor_data,
        );

        // 2. Spawn existing players for our new Bedrock client
        let players = self.players.load();

        for existing_player in players
            .iter()
            .filter(|p| p.gameprofile.id != gameprofile.id)
        {
            let ex_profile = &existing_player.gameprofile;
            let ex_entity = &existing_player.get_entity();
            let ex_pos = ex_entity.pos.load();
            let ex_vel = ex_entity.velocity.load();

            let ex_player_list = CPlayerList {
                action: CPlayerList::ACTION_ADD,
                entries: vec![PlayerListEntry {
                    uuid: ex_profile.id,
                    entity_unique_id: VarLong(existing_player.entity_id() as i64),
                    username: ex_profile.name.clone(),
                    xuid: String::new(),
                    platform_chat_id: String::new(),
                    build_platform: BuildPlatform::Unknown,
                    skin: (**existing_player.bedrock_skin.load()).clone(),
                    is_teacher: false,
                    is_host: false,
                    is_sub_client: false,
                    player_color: [0, 0, 0, 0],
                }],
            };
            // Send PlayerList FIRST
            client.send_packet(&ex_player_list).await;

            let ex_add_player = CAddPlayer {
                uuid: ex_profile.id,
                player_name: ex_profile.name.clone(),
                target_runtime_id: VarULong(existing_player.entity_id() as u64),
                platform_chat_id: String::new(),
                position: Vector3::new(ex_pos.x as f32, ex_pos.y as f32, ex_pos.z as f32),
                velocity: Vector3::new(ex_vel.x as f32, ex_vel.y as f32, ex_vel.z as f32),
                rotation: Vector2::new(ex_entity.pitch.load(), ex_entity.yaw.load()),
                y_head_rotation: ex_entity.head_yaw.load(),
                carried_item: NetworkItemStackDescriptor::default(),
                player_game_type: existing_player.gamemode.load().into(),
                entity_data: ex_entity.bedrock_metadata(),
                synced_properties: PropertySyncData::default(),
                abilities_data: pumpkin_protocol::bedrock::client::SerializedAbilitiesData {
                    target_player_raw_id: existing_player.entity_id() as i64,
                    player_permissions:
                        pumpkin_protocol::bedrock::client::PlayerPermissionLevel::Visitor,
                    command_permissions:
                        pumpkin_protocol::bedrock::client::CommandPermissionLevel::Any,
                    layers: vec![
                        pumpkin_protocol::bedrock::client::SerializedAbilitiesDataSerializedLayer {
                            serialized_layer: 0,
                            abilities_set: 0,
                            ability_value: 0,
                            fly_speed: 0.05,
                            vertical_fly_speed: 0.05,
                            walk_speed: 0.1,
                        },
                    ],
                },
                actor_links: Vec::new(),
                device_id: String::new(),
                build_platform: BuildPlatform::Unknown,
            };

            client.send_packet(&ex_add_player).await;

            let ex_held_item = existing_player.inventory().held_item();

            let ex_be_mob_equipment = pumpkin_protocol::bedrock::client::CMobEquipment {
                target_runtime_id: (existing_player.entity_id() as u64).into(),
                item: (&ex_held_item).into(),
                slot: 0,
                selected_slot: 0,
                container_id: 0,
            };

            client.send_packet(&ex_be_mob_equipment).await;
        }

        player.has_played_before.store(true, Ordering::Relaxed);

        // 3. Trigger Join Event and Broadcast Join Message
        let msg_comp = TextComponent::translate_cross(
            translation::java::MULTIPLAYER_PLAYER_JOINED,
            translation::bedrock::MULTIPLAYER_PLAYER_JOINED,
            [TextComponent::text(player.gameprofile.name.clone())],
        )
        .color_named(NamedColor::Yellow);

        let mut event = PlayerJoinEvent::new(player.clone(), msg_comp);
        server.plugin_manager.fire(server, &mut event).await;

        if !event.cancelled {
            self.broadcast_system_message(&event.join_message, false);
            info!("{}", event.join_message.to_pretty_console());
        }
    }

    #[expect(clippy::too_many_lines)]
    pub async fn spawn_java_player(
        &self,
        base_config: &BasicConfiguration,
        player: &Arc<Player>,
        server: &Arc<Server>,
    ) {
        let dimensions: Vec<ResourceLocation> = server
            .dimensions
            .iter()
            .map(|d| ResourceLocation::from(d.minecraft_name))
            .collect();

        // This code follows the vanilla packet order
        let entity_id = player.entity_id();
        let gamemode = player.gamemode.load();
        debug!(
            "spawning player {}, entity id {}",
            player.gameprofile.name, entity_id
        );

        let Some(client) = player.client.java() else {
            return;
        };
        // Send the login packet for our new player
        client
            .send_packet(&CLogin::new(
                entity_id,
                base_config.hardcore,
                &dimensions,
                server
                    .advanced_config
                    .networking
                    .java
                    .max_players
                    .try_into()
                    .unwrap_or(u16::MAX.into()),
                server
                    .advanced_config
                    .networking
                    .java
                    .view_distance
                    .get()
                    .into(), //  TODO: view distance
                server
                    .advanced_config
                    .networking
                    .java
                    .simulation_distance
                    .get()
                    .into(), // TODO: sim view dinstance
                false,
                true,
                false,
                PlayerSpawnData::new(
                    self.dimension.clone(),
                    biome::hash_seed(self.level.seed.0), // seed
                    gamemode as u8,
                    player
                        .previous_gamemode
                        .load()
                        .map_or(-1, |gamemode| gamemode as i8),
                    false,
                    false,
                    None,
                    VarInt(player.get_entity().portal_cooldown.load(Ordering::Relaxed) as i32),
                    self.sea_level.into(),
                ),
                server.advanced_config.networking.java.online_mode,
                // This should stay true even when reports are disabled.
                // It prevents the annoying popup when joining the server.
                true,
            ))
            .await;

        self.pair_new_player_with_tracked_entities(player);

        // Send the current ticking state to the new player so they are in sync.
        server.tick_rate_manager.update_joining_player(player).await;

        // Permissions, i.e. the commands a player may use.
        player.send_permission_lvl_update();

        // Difficulty of the world
        player.send_difficulty_update();
        {
            let command_dispatcher = server.command_dispatcher.load();

            client_suggestions::send_c_commands_packet(player, server, &command_dispatcher);
        };
        if client.version.load() < JavaMinecraftVersion::V_1_20_2
            && client.version.load() >= JavaMinecraftVersion::V_1_13
        {
            let version = client.version.load();
            let mut tags = Vec::new();
            for &key in pumpkin_data::tag::RegistryKey::NETWORK_KEYS {
                if pumpkin_data::tag::get_registry_key_tags(version, key)
                    .is_some_and(|map| !map.is_empty())
                {
                    tags.push(key);
                }
            }
            let packet = pumpkin_protocol::java::client::play::CUpdateTagsPlay::new(&tags);
            if let Ok(packet_data) = JavaClient::serialize_packet_for_version(&packet, version) {
                client.send_packet_now(packet_data).await;
            }
        }

        let (position, yaw, pitch) = if player.has_played_before.load(Ordering::Relaxed) {
            let position = player.position();
            let yaw = player.get_entity().yaw.load(); //info.spawn_angle;
            let pitch = player.get_entity().pitch.load();

            (position, yaw, pitch)
        } else {
            let info = &self.level_info.load();
            let spawn_position = Vector2::new(info.spawn_x, info.spawn_z);
            let chunk_pos = Vector2::new(info.spawn_x >> 4, info.spawn_z >> 4);
            self.level.get_or_fetch_chunk(chunk_pos, |_| ()).await;
            let top = self.get_top_block(spawn_position);
            let pos_y = if top > self.dimension.min_y {
                top + 1
            } else {
                info.spawn_y
            };

            let position = Vector3::new(
                f64::from(info.spawn_x) + 0.5,
                f64::from(pos_y),
                f64::from(info.spawn_z) + 0.5,
            );
            (position, info.spawn_yaw, info.spawn_pitch)
        };

        // Load chunks around the real spawn position before teleporting the client there.
        player.living_entity.entity.set_pos(position);
        player.living_entity.entity.set_rotation(yaw, pitch);
        player.living_entity.entity.last_pos.store(position);
        chunker::update_position(player);

        let center_chunk = player.living_entity.entity.chunk_pos.load();
        let chunk = self
            .level
            .get_or_fetch_chunk(center_chunk, std::clone::Clone::clone)
            .await;
        if let Some(server) = self.server.upgrade() {
            let mut event =
                crate::plugin::world::chunk_send::ChunkSend::new(player.world(), chunk.clone());
            server.plugin_manager.fire(&server, &mut event).await;
            if event.cancelled {
                return;
            }
        }
        client.send_chunks(&[chunk]).await;

        let velocity = player.living_entity.entity.velocity.load();

        debug!("Sending player teleport to {}", player.gameprofile.name);
        player.request_teleport(position, yaw, pitch);

        let gameprofile = &player.gameprofile;
        let bedrock_player_list = CPlayerList {
            action: CPlayerList::ACTION_ADD,
            entries: vec![PlayerListEntry {
                uuid: gameprofile.id,
                entity_unique_id: VarLong(entity_id as i64),
                username: gameprofile.name.clone(),
                xuid: String::new(),
                platform_chat_id: String::new(),
                build_platform: BuildPlatform::Unknown,
                skin: (**player.bedrock_skin.load()).clone(),
                is_teacher: false,
                is_host: false,
                is_sub_client: false,
                player_color: [0, 0, 0, 0],
            }],
        };

        let player_actions = [
            PlayerAction::AddPlayer {
                name: &gameprofile.name,
                properties: &gameprofile.properties.load(),
            },
            PlayerAction::UpdateGameMode(VarInt(gamemode as i32)),
            PlayerAction::UpdateListed(true),
            PlayerAction::UpdateLatency(VarInt(0)),
            PlayerAction::UpdateListOrder(VarInt(0)),
            PlayerAction::UpdateHat(true),
        ];
        let java_player = [pumpkin_protocol::java::client::play::Player {
            uuid: gameprofile.id,
            actions: &player_actions,
        }];
        let player_info_update = CPlayerInfoUpdate::new(
            (PlayerInfoFlags::ADD_PLAYER
                | PlayerInfoFlags::UPDATE_GAME_MODE
                | PlayerInfoFlags::UPDATE_LISTED
                | PlayerInfoFlags::UPDATE_LATENCY
                | PlayerInfoFlags::UPDATE_LIST_PRIORITY
                | PlayerInfoFlags::UPDATE_HAT)
                .bits(),
            &java_player,
        );

        self.broadcast_editioned(&player_info_update, &bedrock_player_list);

        // If the player has a custom tab_list_name, send an update for it
        if let Some(tab_list_name) = player.get_tab_list_name() {
            let actions = [PlayerAction::UpdateDisplayName(Some(&tab_list_name))];
            let java_player = [pumpkin_protocol::java::client::play::Player {
                uuid: gameprofile.id,
                actions: &actions,
            }];
            self.broadcast_packet_all(&CPlayerInfoUpdate::new(
                PlayerInfoFlags::UPDATE_DISPLAY_NAME.bits(),
                &java_player,
            ));
        }

        // Here, we send all the infos of players who already joined.
        let mut players_tab_list_names = Vec::new();
        {
            let players = self.players.load();
            let mut data_to_process = Vec::new();
            for p in players
                .iter()
                .filter(|p| p.gameprofile.id != player.gameprofile.id)
            {
                let props_guard = p.gameprofile.properties.load();
                data_to_process.push((props_guard, p));
            }

            let mut current_player_data = Vec::new();
            for (properties, player) in &data_to_process {
                let chat_session = player
                    .chat_session
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                let tab_list_name = player.get_tab_list_name();

                let mut player_actions = vec![PlayerAction::AddPlayer {
                    name: &player.gameprofile.name,
                    properties,
                }];

                if base_config.allow_chat_reports {
                    let initialized = chat_session.session_id != uuid::Uuid::nil()
                        && !chat_session.public_key.is_empty()
                        && !chat_session.signature.is_empty();
                    player_actions.push(PlayerAction::InitializeChat(initialized.then(|| {
                        InitChat {
                            session_id: chat_session.session_id,
                            expires_at: chat_session.expires_at,
                            public_key: chat_session.public_key.clone(),
                            signature: chat_session.signature.clone(),
                        }
                    })));
                }

                player_actions.extend([
                    PlayerAction::UpdateGameMode(VarInt(player.gamemode.load() as i32)),
                    PlayerAction::UpdateListed(player.tab_list_listed.load(Ordering::Relaxed)),
                    PlayerAction::UpdateLatency(VarInt(
                        player.tab_list_latency.load(Ordering::Relaxed),
                    )),
                    PlayerAction::UpdateListOrder(VarInt(
                        player.tab_list_order.load(Ordering::Relaxed),
                    )),
                    PlayerAction::UpdateHat(true),
                ]);
                drop(chat_session);

                current_player_data.push((&player.gameprofile.id, player_actions));

                // Collect tab_list_names for sending later
                if tab_list_name.is_some() {
                    players_tab_list_names.push((player.gameprofile.id, tab_list_name));
                }
            }

            let mut action_flags = PlayerInfoFlags::ADD_PLAYER
                | PlayerInfoFlags::UPDATE_LISTED
                | PlayerInfoFlags::UPDATE_LATENCY
                | PlayerInfoFlags::UPDATE_LIST_PRIORITY
                | PlayerInfoFlags::UPDATE_GAME_MODE
                | PlayerInfoFlags::UPDATE_HAT;
            if base_config.allow_chat_reports {
                action_flags |= PlayerInfoFlags::INITIALIZE_CHAT;
            }

            let entries = current_player_data
                .iter()
                .map(|(id, actions)| java::client::play::Player {
                    uuid: **id,
                    actions,
                })
                .collect::<Vec<_>>();

            debug!("Sending player info to {}", player.gameprofile.name);
            client
                .enqueue_client_packet(&CPlayerInfoUpdate::new(action_flags.bits(), &entries))
                .await;

            // Send tab_list_names for existing players with custom names
            for (player_id, tab_list_name) in &players_tab_list_names {
                if let Some(name) = tab_list_name {
                    let actions = [PlayerAction::UpdateDisplayName(Some(name))];
                    let java_player = [pumpkin_protocol::java::client::play::Player {
                        uuid: *player_id,
                        actions: &actions,
                    }];
                    client
                        .enqueue_client_packet(&CPlayerInfoUpdate::new(
                            PlayerInfoFlags::UPDATE_DISPLAY_NAME.bits(),
                            &java_player,
                        ))
                        .await;
                }
            }
        };

        let gameprofile = &player.gameprofile;

        let bedrock_add_player = CAddPlayer {
            uuid: gameprofile.id,
            player_name: gameprofile.name.clone(),
            target_runtime_id: VarULong(entity_id as u64),
            platform_chat_id: String::new(),
            position: Vector3::new(position.x as f32, position.y as f32, position.z as f32),
            velocity: Vector3::new(velocity.x as f32, velocity.y as f32, velocity.z as f32),
            rotation: Vector2::new(pitch, yaw),
            y_head_rotation: yaw,
            carried_item: NetworkItemStackDescriptor::default(),
            player_game_type: player.gamemode.load().into(),
            entity_data: player.get_entity().bedrock_metadata(),
            synced_properties: PropertySyncData::default(),
            abilities_data: pumpkin_protocol::bedrock::client::SerializedAbilitiesData {
                target_player_raw_id: entity_id as i64,
                player_permissions:
                    pumpkin_protocol::bedrock::client::PlayerPermissionLevel::Visitor,
                command_permissions: pumpkin_protocol::bedrock::client::CommandPermissionLevel::Any,
                layers: vec![
                    pumpkin_protocol::bedrock::client::SerializedAbilitiesDataSerializedLayer {
                        serialized_layer: 0,
                        abilities_set: 0,
                        ability_value: 0,
                        fly_speed: 0.05,
                        vertical_fly_speed: 0.05,
                        walk_speed: 0.1,
                    },
                ],
            },
            actor_links: Vec::new(),
            device_id: String::new(),
            build_platform: BuildPlatform::Unknown,
        };

        // Spawn the player for every client.
        let spawn_entity = CSpawnEntity::new(
            entity_id.into(),
            gameprofile.id,
            i32::from(EntityType::PLAYER.id).into(),
            position,
            pitch,
            yaw,
            yaw,
            0.into(),
            velocity,
        );

        self.broadcast_packet_except_editioned(
            &[player.gameprofile.id],
            &spawn_entity,
            &bedrock_add_player,
        );

        // Broadcast metadata to Java players so they can correctly interact with the new player
        let skin_parts = player.config.load().skin_parts;

        self.broadcast_skin_parts(
            &[gameprofile.id],
            entity_id,
            skin_parts,
            &CSetActorData {
                target_runtime_id: VarULong(entity_id as u64),
                actor_data: player.get_entity().bedrock_metadata(),
                synced_properties: PropertySyncData {
                    int_entries_list: HashMap::new(),
                    float_entries_list: HashMap::new(),
                },
                tick: VarULong(0),
            },
        );

        // Spawn players for our client.
        let id = player.gameprofile.id;
        for existing_player in self
            .players
            .load()
            .iter()
            .filter(|c| c.gameprofile.id != id)
        {
            let entity = &existing_player.get_entity();
            let pos = entity.pos.load();
            let gameprofile = &existing_player.gameprofile;
            let bedrock_add_player = CAddPlayer {
                uuid: gameprofile.id,
                player_name: gameprofile.name.clone(),
                target_runtime_id: VarULong(existing_player.entity_id() as u64),
                platform_chat_id: String::new(),
                position: Vector3::new(pos.x as f32, pos.y as f32, pos.z as f32),
                velocity: Vector3::new(
                    entity.velocity.load().x as f32,
                    entity.velocity.load().y as f32,
                    entity.velocity.load().z as f32,
                ),
                rotation: Vector2::new(entity.pitch.load(), entity.yaw.load()),
                y_head_rotation: entity.head_yaw.load(),
                carried_item: NetworkItemStackDescriptor::default(),
                player_game_type: existing_player.gamemode.load().into(),
                entity_data: entity.bedrock_metadata(),
                synced_properties: PropertySyncData::default(),
                abilities_data: pumpkin_protocol::bedrock::client::SerializedAbilitiesData {
                    target_player_raw_id: existing_player.entity_id() as i64,
                    player_permissions:
                        pumpkin_protocol::bedrock::client::PlayerPermissionLevel::Visitor,
                    command_permissions:
                        pumpkin_protocol::bedrock::client::CommandPermissionLevel::Any,
                    layers: vec![
                        pumpkin_protocol::bedrock::client::SerializedAbilitiesDataSerializedLayer {
                            serialized_layer: 0,
                            abilities_set: 0,
                            ability_value: 0,
                            fly_speed: 0.05,
                            vertical_fly_speed: 0.05,
                            walk_speed: 0.1,
                        },
                    ],
                },
                actor_links: Vec::new(),
                device_id: String::new(),
                build_platform: BuildPlatform::Unknown,
            };

            let bedrock_player_list = CPlayerList {
                action: CPlayerList::ACTION_ADD,
                entries: vec![PlayerListEntry {
                    uuid: gameprofile.id,
                    entity_unique_id: VarLong(existing_player.entity_id() as i64),
                    username: gameprofile.name.clone(),
                    xuid: String::new(),
                    platform_chat_id: String::new(),
                    build_platform: BuildPlatform::Unknown,
                    skin: (**existing_player.bedrock_skin.load()).clone(),
                    is_teacher: false,
                    is_host: false,
                    is_sub_client: false,
                    player_color: [0, 0, 0, 0],
                }],
            };

            let actions = [
                PlayerAction::AddPlayer {
                    name: &gameprofile.name,
                    properties: &gameprofile.properties.load(),
                },
                PlayerAction::UpdateGameMode(VarInt(existing_player.gamemode.load() as i32)),
                PlayerAction::UpdateListed(existing_player.tab_list_listed.load(Ordering::Relaxed)),
                PlayerAction::UpdateLatency(VarInt(
                    existing_player.tab_list_latency.load(Ordering::Relaxed),
                )),
                PlayerAction::UpdateListOrder(VarInt(
                    existing_player.tab_list_order.load(Ordering::Relaxed),
                )),
                PlayerAction::UpdateHat(true),
            ];
            let java_player = [pumpkin_protocol::java::client::play::Player {
                uuid: gameprofile.id,
                actions: &actions,
            }];
            player
                .client
                .enqueue_packet_editioned(
                    &CPlayerInfoUpdate::new(
                        (PlayerInfoFlags::ADD_PLAYER
                            | PlayerInfoFlags::UPDATE_LISTED
                            | PlayerInfoFlags::UPDATE_GAME_MODE
                            | PlayerInfoFlags::UPDATE_LATENCY
                            | PlayerInfoFlags::UPDATE_LIST_PRIORITY
                            | PlayerInfoFlags::UPDATE_HAT)
                            .bits(),
                        &java_player,
                    ),
                    &bedrock_player_list,
                )
                .await;

            player
                .client
                .enqueue_packet_editioned(
                    &CSpawnEntity::new(
                        existing_player.entity_id().into(),
                        gameprofile.id,
                        i32::from(EntityType::PLAYER.id).into(),
                        pos,
                        entity.pitch.load(),
                        entity.yaw.load(),
                        entity.head_yaw.load(),
                        0.into(),
                        entity.velocity.load(),
                    ),
                    &bedrock_add_player,
                )
                .await;

            if client.version.load() >= JavaMinecraftVersion::V_1_21 {
                let config = existing_player.config.load();
                let mut buf = Vec::new();
                {
                    let meta = Metadata::new(
                        pumpkin_data::tracked_data::player::PLAYER_MODE_CUSTOMISATION,
                        config.skin_parts,
                    );
                    let _ = meta.write(&mut buf, &client.version.load());
                };
                {
                    let meta = Metadata::new(
                        pumpkin_data::tracked_data::player::PLAYER_MODE_CUSTOMIZATION_ID,
                        config.skin_parts,
                    );
                    let _ = meta.write(&mut buf, &client.version.load());
                };
                drop(config);
                // END
                buf.put_u8(255);
                client
                    .enqueue_client_packet(&CSetEntityMetadata::new(
                        existing_player.get_entity().entity_id.into(),
                        buf.into(),
                    ))
                    .await;
            }

            {
                let held_item = existing_player.inventory.held_item();
                let equipment_list = {
                    let mut equipment_list =
                        vec![(EquipmentSlot::MAIN_HAND.discriminant(), held_item.clone())];

                    let equipment_guard = existing_player
                        .inventory
                        .entity_equipment
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    for (slot, item_stack) in &equipment_guard.equipment {
                        equipment_list.push((slot.discriminant(), item_stack.clone()));
                    }
                    equipment_list
                };

                let equipment: Vec<(i8, ItemStackSerializer)> = equipment_list
                    .iter()
                    .map(|(slot, stack)| (*slot, ItemStackSerializer::from(stack.clone())))
                    .collect();

                let je_packet = CSetEquipment::new(existing_player.entity_id().into(), equipment);

                let be_mob_equipment = pumpkin_protocol::bedrock::client::CMobEquipment {
                    target_runtime_id: (existing_player.entity_id() as u64).into(),
                    item: (&held_item).into(),
                    slot: 0,
                    selected_slot: 0,
                    container_id: 0,
                };

                player
                    .client
                    .enqueue_packet_editioned(&je_packet, &be_mob_equipment)
                    .await;
            }
        }
        player.send_client_information();

        player.send_abilities_update();

        // Sync selected slot
        player.enqueue_set_held_item_packet(&CSetSelectedSlot::new(
            player.get_inventory().get_selected_slot() as i8,
        ));

        if client.version.load() >= JavaMinecraftVersion::V_1_20_2 {
            // Start waiting for level chunks. Sets the "Loading Terrain" screen (Added in 1.20.2)
            debug!("Sending waiting chunks to {}", player.gameprofile.name);
            client
                .send_packet(&CGameEvent::new(GameEvent::StartWaitingChunks, 0.0))
                .await;
        }

        self.worldborder
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .init_client(client);

        // Sends initial time
        player.send_time(self);

        // Sends initial scoreboard state
        player.send_scoreboard();

        let (spawn_block_pos, yaw, pitch) = {
            let level_info_lock = self.level_info.load();
            (
                BlockPos::new(
                    level_info_lock.spawn_x,
                    level_info_lock.spawn_y,
                    level_info_lock.spawn_z,
                ),
                level_info_lock.spawn_yaw,
                level_info_lock.spawn_pitch,
            )
        };

        client
            .send_packet(&CPlayerSpawnPosition::new(
                spawn_block_pos,
                yaw,
                pitch,
                self.dimension.minecraft_name.to_owned(),
            ))
            .await;

        // Send initial weather state
        let (is_raining, rain_level, thunder_level) = {
            let weather = self
                .weather
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            (
                weather.raining,
                weather.rain_level.clamp(0.0, 1.0),
                weather.thunder_level.clamp(0.0, 1.0),
            )
        };
        if is_raining {
            client
                .enqueue_client_packet(&CGameEvent::new(GameEvent::BeginRaining, 0.0))
                .await;

            client
                .enqueue_client_packet(&CGameEvent::new(GameEvent::RainLevelChange, rain_level))
                .await;
            client
                .enqueue_client_packet(&CGameEvent::new(
                    GameEvent::ThunderLevelChange,
                    thunder_level,
                ))
                .await;
        }

        let player_bossbars = server
            .bossbars
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get_player_bars(&player.gameprofile.id)
            .map(|bars| bars.into_iter().cloned().collect::<Vec<_>>());
        if let Some(bossbars) = player_bossbars {
            for bossbar in &bossbars {
                player.send_bossbar(bossbar);
            }
        }

        player.has_played_before.store(true, Ordering::Relaxed);
        player.on_screen_handler_opened(&player.player_screen_handler);

        player.send_active_effects();
        player.breath_manager.send_air_supply(player);
        self.send_player_equipment(player);

        if let crate::net::ClientPlatform::Java(java_client) = player.client.as_ref()
            && server.advanced_config.recipe.send_recipes
            && java_client.version.load() >= JavaMinecraftVersion::V_1_21_2
        {
            let settings_packet = CRecipeBookSettings::default_closed();
            if let Ok(data) = java_client.serialize_packet(&settings_packet) {
                java_client.send_packet_now(data).await;
            }
            let dynamic_recipes = server.recipe_manager.get_dynamic_recipes();
            let add_packet = CRecipeBookAdd::new(true, &dynamic_recipes);
            if let Ok(data) = java_client.serialize_packet(&add_packet) {
                java_client.send_packet_now(data).await;
            }
        }
        let msg_comp = TextComponent::translate_cross(
            translation::java::MULTIPLAYER_PLAYER_JOINED,
            translation::bedrock::MULTIPLAYER_PLAYER_JOINED,
            [TextComponent::text(player.gameprofile.name.clone())],
        )
        .color_named(NamedColor::Yellow);
        let mut event = PlayerJoinEvent::new(player.clone(), msg_comp);

        server.plugin_manager.fire(server, &mut event).await;

        if !event.cancelled {
            self.broadcast_system_message(&event.join_message, false);
            // TODO: Switch to structured logging, e.g. info!(player = %name, "connected")
            info!("{}", event.join_message.to_pretty_console());
        }
    }

    fn send_player_equipment(&self, from: &Player) {
        let held_item = from.inventory.held_item();
        let mut equipment_list = vec![(EquipmentSlot::MAIN_HAND.discriminant(), held_item.clone())];

        let equipment_guard = from
            .inventory
            .entity_equipment
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        for (slot, item_stack) in &equipment_guard.equipment {
            equipment_list.push((slot.discriminant(), item_stack.clone()));
        }
        drop(equipment_guard);

        let equipment: Vec<(i8, ItemStackSerializer)> = equipment_list
            .iter()
            .map(|(slot, stack)| (*slot, ItemStackSerializer::from(stack.clone())))
            .collect();
        let je_packet = CSetEquipment::new(from.entity_id().into(), equipment);

        let be_mob_equipment = pumpkin_protocol::bedrock::client::CMobEquipment {
            target_runtime_id: (from.entity_id() as u64).into(),
            item: (&held_item).into(),
            slot: 0,
            selected_slot: 0,
            container_id: 0,
        };

        self.send_to_tracking_players_editioned(from.get_entity(), &je_packet, &be_mob_equipment);
    }

    pub fn send_world_info(
        &self,
        player: &Arc<Player>,
        position: Vector3<f64>,
        yaw: f32,
        pitch: f32,
    ) {
        if let ClientPlatform::Java(client) = player.client.as_ref() {
            self.worldborder
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .init_client(client);
        }

        // TODO: World spawn (compass stuff)

        if let ClientPlatform::Java(client) = player.client.as_ref()
            && client.version.load() >= JavaMinecraftVersion::V_1_20_2
        {
            player.try_send_client_packet(&CGameEvent::new(GameEvent::StartWaitingChunks, 0.0));
        }

        let entity = &player.get_entity();

        self.broadcast_packet_except(
            &[player.gameprofile.id],
            // TODO: add velo
            &CSpawnEntity::new(
                entity.entity_id.into(),
                player.gameprofile.id,
                i32::from(EntityType::PLAYER.id).into(),
                position,
                pitch,
                yaw,
                yaw,
                0.into(),
                Vector3::new(0.0, 0.0, 0.0),
            ),
        );

        player.send_client_information();

        chunker::update_position(player);
        // Update commands

        player.set_health(20.0);
    }

    pub fn explode(
        self: &Arc<Self>,
        position: Vector3<f64>,
        power: f32,
        interaction: ExplosionInteraction,
    ) {
        self.explode_with_calculator(position, power, interaction, None);
    }

    pub fn explode_with_calculator(
        self: &Arc<Self>,
        position: Vector3<f64>,
        power: f32,
        interaction: ExplosionInteraction,
        damage_calculator: Option<Arc<dyn ExplosionDamageCalculator>>,
    ) {
        let block_interaction = self.get_block_interaction(interaction);
        let mut explosion = Explosion::new(power, position, block_interaction);
        if let Some(calc) = damage_calculator {
            explosion = explosion.with_damage_calculator(calc);
        }
        self.run_explosion(&explosion, position, power);
    }

    pub fn explode_with_calculator_and_effects(
        self: &Arc<Self>,
        position: Vector3<f64>,
        power: f32,
        interaction: ExplosionInteraction,
        damage_calculator: Option<Arc<dyn ExplosionDamageCalculator>>,
        particle: Option<Particle>,
        sound: Option<Sound>,
        is_wind_charge: bool,
    ) {
        let block_interaction = self.get_block_interaction(interaction);
        let mut explosion = Explosion::new(power, position, block_interaction);
        if let Some(calc) = damage_calculator {
            explosion = explosion.with_damage_calculator(calc);
        }
        if let Some(particle) = particle {
            explosion = explosion.with_particle(particle);
        }
        if let Some(sound) = sound {
            explosion = explosion.with_sound(sound);
        }
        if is_wind_charge {
            explosion = explosion.with_wind_charge(true);
        }
        self.run_explosion(&explosion, position, power);
    }

    pub fn explode_tnt_minecart(self: &Arc<Self>, position: Vector3<f64>, power: f32) {
        let block_interaction = self.get_block_interaction(ExplosionInteraction::Tnt);
        let explosion = Explosion::new(power, position, block_interaction).preserving_rails();
        self.run_explosion(&explosion, position, power);
    }

    #[must_use]
    pub fn get_block_interaction(&self, interaction: ExplosionInteraction) -> BlockInteraction {
        let game_rules = &self.level_info.load().game_rules;
        match interaction {
            ExplosionInteraction::None => BlockInteraction::Keep,
            ExplosionInteraction::Block => {
                Self::get_destroy_type(game_rules.block_explosion_drop_decay)
            }
            ExplosionInteraction::Mob => {
                if game_rules.mob_griefing {
                    Self::get_destroy_type(game_rules.mob_explosion_drop_decay)
                } else {
                    BlockInteraction::Keep
                }
            }
            ExplosionInteraction::Tnt => {
                Self::get_destroy_type(game_rules.tnt_explosion_drop_decay)
            }
            ExplosionInteraction::Trigger => BlockInteraction::TriggerBlock,
        }
    }

    #[must_use]
    pub const fn get_destroy_type(drop_decay: bool) -> BlockInteraction {
        if drop_decay {
            BlockInteraction::DestroyWithDecay
        } else {
            BlockInteraction::Destroy
        }
    }

    fn run_explosion(self: &Arc<Self>, explosion: &Explosion, position: Vector3<f64>, power: f32) {
        let mut event = crate::plugin::api::events::entity::entity_explode::EntityExplodeEvent::new(
            0, position, power,
        );
        if let Some(server) = self.server.upgrade() {
            server.plugin_manager.fire_blocking(&server, &mut event);
        }
        if event.cancelled {
            return;
        }

        let (block_count, player_knockbacks) = explosion.explode(self);
        let particle = explosion.particle().unwrap_or_else(|| {
            if power < 2.0 {
                Particle::Explosion
            } else {
                Particle::ExplosionEmitter
            }
        });
        let sound_event = explosion.sound().unwrap_or(Sound::EntityGenericExplode);
        let sound = IdOr::<SoundEvent>::Id(sound_event as u16);
        for player in self.players.load().iter() {
            if player.position().squared_distance_to_vec(&position) > 4096.0 {
                continue;
            }
            // Pass the per-player knockback vector so the client can apply it client-side
            let knockback = player_knockbacks.get(&player.entity_id()).copied();
            player.try_send_client_packet(&CExplosion::new(
                position,
                power,
                block_count as i32,
                knockback,
                VarInt(particle as i32),
                sound.clone(),
            ));
        }
    }

    pub(crate) fn despawn_dead_java_player_for_bedrock(&self, subject: &Entity) {
        let Some(player) = self.get_player_by_id(subject.entity_id) else {
            return;
        };
        if matches!(player.client.as_ref(), ClientPlatform::Java(_)) {
            self.broadcast_to_chunk_bedrock(
                subject.chunk_pos.load(),
                &CRemoveActor::new(VarLong(subject.entity_id.into())),
            );
        }
    }

    async fn refresh_java_player_for_bedrock(&self, subject: &Player) {
        if !matches!(subject.client.as_ref(), ClientPlatform::Java(_)) {
            return;
        }

        let entity = subject.get_entity();
        let entity_id = subject.entity_id();
        let position = entity.pos.load();
        let velocity = entity.velocity.load();
        let player_list = CPlayerList {
            action: CPlayerList::ACTION_ADD,
            entries: vec![PlayerListEntry {
                uuid: subject.gameprofile.id,
                entity_unique_id: VarLong(entity_id.into()),
                username: subject.gameprofile.name.clone(),
                xuid: String::new(),
                platform_chat_id: String::new(),
                build_platform: BuildPlatform::Unknown,
                skin: (**subject.bedrock_skin.load()).clone(),
                is_teacher: false,
                is_host: false,
                is_sub_client: false,
                player_color: [0; 4],
            }],
        };
        let add_player = CAddPlayer {
            uuid: subject.gameprofile.id,
            player_name: subject.gameprofile.name.clone(),
            target_runtime_id: VarULong(entity_id as u64),
            platform_chat_id: String::new(),
            position: Vector3::new(position.x as f32, position.y as f32, position.z as f32),
            velocity: Vector3::new(velocity.x as f32, velocity.y as f32, velocity.z as f32),
            rotation: Vector2::new(entity.pitch.load(), entity.yaw.load()),
            y_head_rotation: entity.head_yaw.load(),
            carried_item: NetworkItemStackDescriptor::default(),
            player_game_type: subject.gamemode.load().into(),
            entity_data: entity.bedrock_metadata(),
            synced_properties: PropertySyncData::default(),
            abilities_data: pumpkin_protocol::bedrock::client::SerializedAbilitiesData {
                target_player_raw_id: entity_id as i64,
                player_permissions:
                    pumpkin_protocol::bedrock::client::PlayerPermissionLevel::Visitor,
                command_permissions: pumpkin_protocol::bedrock::client::CommandPermissionLevel::Any,
                layers: vec![
                    pumpkin_protocol::bedrock::client::SerializedAbilitiesDataSerializedLayer {
                        serialized_layer: 0,
                        abilities_set: 0,
                        ability_value: 0,
                        fly_speed: 0.05,
                        vertical_fly_speed: 0.05,
                        walk_speed: 0.1,
                    },
                ],
            },
            actor_links: Vec::new(),
            device_id: String::new(),
            build_platform: BuildPlatform::Unknown,
        };
        let remove = CRemoveActor::new(VarLong(entity_id.into()));

        for recipient in self.players.load().iter() {
            if let ClientPlatform::Bedrock(client) = recipient.client.as_ref() {
                client.send_packet(&remove).await;
                client.send_packet(&player_list).await;
                client.send_packet(&add_player).await;
            }
        }
    }

    #[allow(clippy::too_many_lines)]
    pub async fn respawn_player(self: &Arc<Self>, player: &Arc<Player>, alive: bool) {
        let last_pos = player.get_entity().last_pos.load();
        let death_dimension = ResourceLocation::from(player.world().dimension.minecraft_name);
        let death_location = BlockPos(Vector3::new(
            last_pos.x.round() as i32,
            last_pos.y.round() as i32,
            last_pos.z.round() as i32,
        ));

        let data_kept = u8::from(alive);

        let server = self.server.upgrade();
        let default_world = server.as_ref().map_or_else(
            || self.clone(),
            |s| s.get_world_from_dimension(&Dimension::OVERWORLD),
        );

        // Copy spawn info from default world level_info to avoid holding lock across await
        let (spawn_x, spawn_y, spawn_z, spawn_yaw, spawn_pitch, keep_inventory) = {
            let info = default_world.level_info.load();
            (
                info.spawn_x,
                info.spawn_y,
                info.spawn_z,
                info.spawn_yaw,
                info.spawn_pitch,
                info.game_rules.keep_inventory,
            )
        };

        // Get respawn position and dimension
        let (position, yaw, pitch, respawn_dimension) = if let Some(respawn) =
            player.calculate_respawn_point().await
        {
            (
                respawn.position,
                respawn.yaw,
                respawn.pitch,
                respawn.dimension,
            )
        } else {
            // No valid respawn point - send notification if player had one set
            if player
                .respawn_point
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .is_some()
            {
                player
                    .send_client_packet(&CGameEvent::new(GameEvent::NoRespawnBlockAvailable, 0.0))
                    .await;
                let mut guard = player
                    .respawn_point
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                if let Some(point) = guard.as_ref()
                    && !point.force
                {
                    *guard = None;
                }
            }

            // FIXME: This spawn position calculation is incorrect. Should use vanilla's
            // proper spawn position calculation (see #1381). The y-level calculation
            // needs to account for spawn radius and find a safe spawn position.
            let chunk_pos = Vector2::new(spawn_x >> 4, spawn_z >> 4);
            default_world
                .level
                .get_or_fetch_chunk(chunk_pos, |_| ())
                .await;
            let top = default_world.get_top_block(Vector2::new(spawn_x, spawn_z));
            let pos_y = if top > default_world.dimension.min_y {
                top + 1
            } else {
                spawn_y
            };

            (
                Vector3::new(
                    f64::from(spawn_x) + 0.5,
                    f64::from(pos_y),
                    f64::from(spawn_z) + 0.5,
                ),
                spawn_yaw,
                spawn_pitch,
                default_world.dimension.clone(),
            )
        };

        let mut spawn_loc_event = crate::plugin::api::events::player::player_spawn_location::PlayerSpawnLocationEvent::new(
            player.clone(),
            position,
        );
        if let Some(ref s) = server {
            s.plugin_manager.fire(s, &mut spawn_loc_event).await;
        }
        let position = spawn_loc_event.spawn_pos;

        // Candidate destination world for a cross-dimension respawn.
        let candidate_world = if respawn_dimension == self.dimension {
            None
        } else {
            server.as_ref().map_or_else(
                || {
                    warn!("Could not get server for cross-dimension respawn");
                    None
                },
                |s| {
                    let worlds = s.worlds.load();
                    worlds
                        .iter()
                        .find(|w| w.dimension == respawn_dimension)
                        .cloned()
                },
            )
        };

        // Fire PlayerChangeWorldEvent (cancellable) before the transfer; it runs before
        // the non-cancellable PlayerRespawnEvent, which observes the resolved world.
        let (resolved_world, position, yaw, pitch) = if let Some(new_world) = candidate_world {
            if let Some(ref s) = server {
                let mut event = PlayerChangeWorldEvent {
                    player: player.clone(),
                    previous_world: self.clone(),
                    new_world: new_world.clone(),
                    position,
                    yaw,
                    pitch,
                    cancelled: false,
                };
                s.plugin_manager.fire(s, &mut event).await;

                if event.cancelled {
                    (None, position, yaw, pitch)
                } else {
                    let destination = event.new_world;
                    let position = event.position;
                    let yaw = event.yaw;
                    let pitch = event.pitch;

                    // Skip the transfer if redirected back to the current world.
                    if destination.uuid != self.uuid {
                        debug!(
                            "Cross-dimension respawn: {} -> {}",
                            self.dimension.minecraft_name, destination.dimension.minecraft_name
                        );

                        // Detach from the old world before publishing into the new one, so no
                        // observer sees the player in a world whose chunk manager doesn't match.
                        self.remove_player(player, false).await;
                        player.unload_watched_chunks(self).await;
                        player.change_world_chunks(&self.level, &destination);
                        player.living_entity.entity.set_world(destination.clone());
                        destination.players.rcu(|current_list| {
                            let mut new_list = (**current_list).clone();
                            new_list.push(player.clone());
                            new_list
                        });
                    }

                    (Some(destination), position, yaw, pitch)
                }
            } else {
                warn!("Server dropped during cross-dimension respawn");
                (None, position, yaw, pitch)
            }
        } else {
            if respawn_dimension != self.dimension {
                warn!(
                    "Target world {:?} not found, using world spawn in {:?}",
                    respawn_dimension, self.dimension
                );
            }
            (None, position, yaw, pitch)
        };

        // Cancelled or unresolved cross-dimension respawns fall back to the current
        // world's spawn below; otherwise the resolved values from the event apply.
        let (target_world, position, yaw, pitch) = resolved_world.as_ref().map_or_else(
            || (self.clone(), position, yaw, pitch),
            |new_world| (new_world.clone(), position, yaw, pitch),
        );

        // Notify plugins that the player has respawned (non-cancellable).
        if let Some(server) = self.server.upgrade() {
            server
                .plugin_manager
                .fire(
                    &server,
                    &mut PlayerRespawnEvent::new(
                        player.clone(),
                        self.clone(),
                        target_world.clone(),
                        position,
                        yaw,
                        pitch,
                        alive,
                    ),
                )
                .await;
        }

        // Send respawn packet with target dimension (using send_packet_now to ensure proper order)
        player
            .send_client_packet(&CRespawn::new(
                PlayerSpawnData::new(
                    target_world.dimension.clone(),
                    biome::hash_seed(target_world.level.seed.0),
                    player.gamemode.load() as u8,
                    player.gamemode.load() as i8,
                    false,
                    false,
                    Some((death_dimension, death_location)),
                    VarInt(player.get_entity().portal_cooldown.load(Ordering::Relaxed) as i32),
                    target_world.sea_level.into(),
                ),
                data_kept,
            ))
            .await;

        // Inform the client of the default spawn position so the client doesn't
        // fall back to (0, 2, 0) while the world reloads (fixes rubberbanding).
        // This must be sent after the CRespawn packet for proper client positioning.
        let spawn_block_pos = BlockPos(Vector3::new(
            position.x.round() as i32,
            position.y.round() as i32,
            position.z.round() as i32,
        ));
        let bedrock_dimension = match target_world.dimension.minecraft_name {
            "minecraft:the_nether" => 1,
            "minecraft:the_end" => 2,
            _ => 0,
        };
        player
            .send_packet_now_editioned(
                &CPlayerSpawnPosition::new(
                    spawn_block_pos,
                    yaw,
                    pitch,
                    target_world.dimension.minecraft_name.to_string(),
                ),
                &pumpkin_protocol::bedrock::client::CSetSpawnPosition {
                    spawn_position_type:
                        pumpkin_protocol::bedrock::client::SpawnPositionType::WorldRespawn,
                    block_position: spawn_block_pos,
                    dimension_type: bedrock_dimension.into(),
                    spawn_block_pos,
                },
            )
            .await;

        player.living_entity.reset_state();

        player.send_permission_lvl_update();

        player.hunger_manager.restart();

        if !keep_inventory {
            player.set_experience(0, 0.0, 0);
            player.inventory.clear();
        }

        // Set entity position BEFORE loading chunks, so chunks load at the right location
        // This mirrors the initial spawn flow where update_position is called before teleport
        player.get_entity().set_pos(position);
        player.get_entity().set_rotation(yaw, pitch);
        player.get_entity().last_pos.store(position);

        // TODO: difficulty, exp bar, status effect

        // Load chunks and send world info FIRST (before teleport packet)
        target_world.send_world_info(player, position, yaw, pitch);

        // Ensure at least the center chunk is sent synchronously before teleport.
        if let crate::net::ClientPlatform::Java(java_client) = player.client.as_ref() {
            let center_chunk = player.get_entity().chunk_pos.load();
            let chunk = target_world
                .level
                .get_or_fetch_chunk(center_chunk, std::clone::Clone::clone)
                .await;
            java_client.send_chunks(&[chunk]).await;
        }

        // Send teleport packet after at least the center chunk was delivered
        player.request_teleport(position, yaw, pitch);

        target_world.refresh_java_player_for_bedrock(player).await;
    }

    /// Returns true if enough players are sleeping and we should skip the night.
    pub fn should_skip_night(&self) -> bool {
        let players = self.players.load();

        let player_count = players.len();
        let sleeping_player_count = players
            .iter()
            .filter(|player| {
                player
                    .sleeping_since
                    .load()
                    .is_some_and(|since| since >= 100)
            })
            .count();
        drop(players);

        if player_count == 0 {
            return false;
        }

        let sleep_percentage = self
            .level_info
            .load()
            .game_rules
            .players_sleeping_percentage
            .clamp(0, 100);
        let required_sleeping =
            ((player_count as f64 * sleep_percentage as f64) / 100.0).ceil() as usize;
        let required_sleeping = required_sleeping.max(1);

        sleeping_player_count >= required_sleeping
    }

    #[must_use]
    pub const fn environment_attributes(&self) -> EnvironmentAttributes<'_> {
        EnvironmentAttributes::new(self)
    }

    pub fn get_biome(&self, position: &BlockPos) -> &'static Biome {
        let chunk_pos = position.chunk_position();
        if let Some(chunk) = self.level.loaded_chunks.get(&chunk_pos) {
            let id = chunk
                .section
                .get_rough_biome_absolute_y(
                    (position.0.x & 15) as usize,
                    position.0.y,
                    (position.0.z & 15) as usize,
                )
                .unwrap_or(0);
            Biome::from_id(id).unwrap_or(&Biome::PLAINS)
        } else {
            &Biome::PLAINS
        }
    }

    /* ItemScatterer.java */
    /* End ItemScatterer.java */
    /// Gets a `Block` from the block registry. Returns `Block::AIR` if the block was not found.
    pub fn get_block(&self, position: &BlockPos) -> &'static Block {
        self.get_block_state_id_if_loaded(position)
            .map_or(&Block::AIR, Block::from_state_id)
    }

    #[must_use]
    pub fn get_block_state_id_if_loaded(&self, position: &BlockPos) -> Option<BlockStateId> {
        if !self.is_in_build_limit(*position) {
            return None;
        }

        let (chunk_coordinate, relative) = position.chunk_and_chunk_relative_position();
        self.level.read_chunk_sync(&chunk_coordinate, |chunk| {
            chunk
                .section
                .get_block_absolute_y(relative.x as usize, relative.y, relative.z as usize)
        })?
    }

    #[must_use]
    pub fn get_block_state_if_loaded(&self, position: &BlockPos) -> Option<&'static BlockState> {
        self.get_block_state_id_if_loaded(position)
            .map(BlockState::from_id)
    }

    #[must_use]
    pub fn is_loaded(&self, position: &BlockPos) -> bool {
        self.get_block_state_id_if_loaded(position).is_some()
    }

    fn get_fluid_from_state_id(id: BlockStateId) -> &'static pumpkin_data::fluid::Fluid {
        if let Some(fluid) = Fluid::from_state_id(id) {
            return fluid.to_flowing();
        }
        if id.is_waterlogged() {
            &Fluid::FLOWING_WATER
        } else {
            &Fluid::EMPTY
        }
    }

    pub fn get_fluid(&self, position: &BlockPos) -> &'static pumpkin_data::fluid::Fluid {
        let id = self.get_block_state_id(position);
        Self::get_fluid_from_state_id(id)
    }

    pub fn get_block_and_fluid(
        &self,
        position: &BlockPos,
    ) -> (
        &'static pumpkin_data::Block,
        &'static pumpkin_data::fluid::Fluid,
    ) {
        let id = self.get_block_state_id(position);
        (id.to_block(), Self::get_fluid_from_state_id(id))
    }

    pub fn get_fluid_and_fluid_state(
        &self,
        position: &BlockPos,
    ) -> (&'static Fluid, &'static FluidState) {
        let id = self.get_block_state_id(position);
        let fluid = Self::get_fluid_from_state_id(id);
        (fluid, &fluid.states[0])
    }

    pub fn get_block_state_id(&self, position: &BlockPos) -> BlockStateId {
        self.get_block_state_id_if_loaded(position)
            .unwrap_or(Block::AIR.default_state.id)
    }

    /// Gets the `BlockState` from the block registry. Returns Air if the block state was not found.
    pub fn get_block_state(&self, position: &BlockPos) -> &'static BlockState {
        let id = self.get_block_state_id(position);
        BlockState::from_id(id)
    }

    /// Gets the Block + Block state from the Block Registry, Returns Air if the Block state has not been found
    pub fn get_block_and_state(
        &self,
        position: &BlockPos,
    ) -> (&'static Block, &'static BlockState) {
        let id = self.get_block_state_id(position);
        BlockState::from_id_with_block(id)
    }

    /// Gets the Block + state id from the Block Registry, Returns Air if the Block state has not been found
    pub fn get_block_and_state_id(&self, position: &BlockPos) -> (&'static Block, BlockStateId) {
        let id = self.get_block_state_id(position);
        (Block::from_state_id(id), id)
    }

    /// Returns whether monsters can be spawned in the world
    pub fn should_spawn_monsters(&self) -> bool {
        let level_data = self.level_info.load();
        level_data.game_rules.spawn_mobs
            && level_data.game_rules.spawn_monsters
            && level_data.difficulty != Difficulty::Peaceful
    }

    pub fn get_block_entity(&self, block_pos: &BlockPos) -> Option<Arc<dyn BlockEntity>> {
        let chunk_pos = block_pos.chunk_position();
        if let Some(entity) = self
            .block_entities
            .get(&chunk_pos)
            .and_then(|m| m.get(block_pos).cloned())
        {
            return Some(entity);
        }

        let nbt = self
            .level
            .read_chunk_sync(&chunk_pos, |chunk| {
                chunk
                    .pending_block_entities
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .get(block_pos)
                    .cloned()
            })
            .flatten()?;
        if let Some(custom_data) = nbt
            .get_compound("PumpkinCustomData")
            .or_else(|| nbt.get_compound("BukkitValues"))
        {
            self.custom_block_entity_data
                .insert(*block_pos, custom_data.clone());
        }
        let entity = block_entity_from_nbt(&nbt)?;
        self.block_entities
            .entry(chunk_pos)
            .or_default()
            .insert(*block_pos, entity.clone());
        Some(entity)
    }

    fn bedrock_block_entity_data(
        &self,
        state_id: BlockStateId,
        position: BlockPos,
    ) -> Option<NbtCompound> {
        self.get_block_entity(&position)?
            .bedrock_block_actor_data(state_id)
    }

    /// Builds Bedrock block actor tags that are not represented by Java block states alone.
    pub fn bedrock_chunk_block_actors(&self, chunk: &ChunkData) -> Vec<NbtCompound> {
        let chunk_pos = Vector2::new(chunk.x, chunk.z);
        let live_entities: FxHashMap<_, _> = self
            .block_entities
            .get(&chunk_pos)
            .map(|entities| {
                entities
                    .iter()
                    .map(|(position, entity)| (*position, entity.clone()))
                    .collect()
            })
            .unwrap_or_default();
        let pending = chunk
            .pending_block_entities
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        live_entities
            .iter()
            .filter_map(|(position, entity)| {
                let relative = position.chunk_relative_position();
                chunk
                    .section
                    .get_block_absolute_y(relative.x as usize, relative.y, relative.z as usize)
                    .and_then(|state_id| {
                        bedrock_chest_block_actor(state_id, *position)
                            .or_else(|| entity.bedrock_block_actor_data(state_id))
                    })
            })
            .chain(
                pending
                    .iter()
                    .filter(|(position, _)| !live_entities.contains_key(position))
                    .filter_map(|(position, nbt)| {
                        let relative = position.chunk_relative_position();
                        let state_id = chunk.section.get_block_absolute_y(
                            relative.x as usize,
                            relative.y,
                            relative.z as usize,
                        )?;
                        bedrock_chest_block_actor(state_id, *position).or_else(|| {
                            block_entity_from_nbt(nbt)?.bedrock_block_actor_data(state_id)
                        })
                    }),
            )
            .collect()
    }

    pub fn add_block_entity(&self, block_entity: Arc<dyn BlockEntity>) {
        let block_pos = block_entity.get_position();
        let chunk_pos = block_pos.chunk_position();
        let block_entity_nbt = block_entity.chunk_data_nbt();
        let entity_id = block_entity.resource_location().to_string();

        if let Some(nbt) = &block_entity_nbt {
            let bytes = pumpkin_nbt::Nbt::from(nbt.clone()).write_unnamed();
            self.broadcast_to_chunk(
                chunk_pos,
                &CBlockEntityData::new(
                    block_entity.get_position(),
                    VarInt(block_entity.get_id() as i32),
                    bytes.as_ref().into(),
                ),
            );
        }

        self.block_entities
            .entry(chunk_pos)
            .or_default()
            .insert(block_pos, block_entity);

        if let Some(nbt) = block_entity_nbt {
            let mut full_nbt = nbt;
            full_nbt.put_string("id", entity_id);
            full_nbt.put_int("x", block_pos.0.x);
            full_nbt.put_int("y", block_pos.0.y);
            full_nbt.put_int("z", block_pos.0.z);
            self.add_block_entity_nbt(block_pos, &full_nbt);
        }

        self.level.read_chunk_sync(&chunk_pos, |chunk| {
            chunk.mark_dirty(true);
        });
    }

    pub(crate) fn add_block_entity_nbt(&self, block_pos: BlockPos, nbt: &NbtCompound) {
        if self
            .level
            .read_chunk_sync(&block_pos.chunk_position(), |chunk| {
                chunk
                    .pending_block_entities
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .insert(block_pos, nbt.clone());
                chunk.mark_dirty(true);
            })
            .is_some()
        {
            self.pending_block_entity_migrations
                .push(block_pos.chunk_position());
        }
    }

    pub fn remove_block_entity(&self, block_pos: &BlockPos) {
        let chunk_pos = block_pos.chunk_position();
        let removed =
            self.block_entities
                .get_mut(&chunk_pos)
                .is_some_and(|mut chunk_block_entities| {
                    chunk_block_entities.remove(block_pos).is_some()
                });
        if removed {
            self.custom_block_entity_data.remove(block_pos);
            // Drop the chunk's map once its last block entity is gone.
            self.block_entities
                .remove_if(&chunk_pos, |_, entities| entities.is_empty());
            self.level.read_chunk_sync(&chunk_pos, |chunk| {
                chunk.mark_dirty(true);
            });
        }
    }

    fn migrate_pending_block_entities(&self, chunk_pos: Vector2<i32>) {
        let positions: Vec<BlockPos> = self
            .level
            .read_chunk_sync(&chunk_pos, |chunk| {
                chunk
                    .pending_block_entities
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .keys()
                    .copied()
                    .collect()
            })
            .unwrap_or_default();
        for pos in positions {
            let already_loaded = self
                .block_entities
                .get(&chunk_pos)
                .is_some_and(|m| m.contains_key(&pos));
            if !already_loaded && let Some(entity) = self.get_block_entity(&pos) {
                self.update_block_entity(&entity);
            }
        }
    }

    pub fn update_block_entity(&self, block_entity: &Arc<dyn BlockEntity>) {
        let block_pos = block_entity.get_position();
        let chunk_pos = block_pos.chunk_position();
        let block_entity_nbt = block_entity.chunk_data_nbt();

        if let Some(nbt) = &block_entity_nbt {
            let bytes = pumpkin_nbt::Nbt::from(nbt.clone()).write_unnamed();
            self.broadcast_to_chunk(
                chunk_pos,
                &CBlockEntityData::new(
                    block_entity.get_position(),
                    VarInt(block_entity.get_id() as i32),
                    bytes.as_ref().into(),
                ),
            );
            let mut full_nbt = nbt.clone();
            full_nbt.put_string("id", block_entity.resource_location().to_string());
            let pos = block_entity.get_position();
            full_nbt.put_int("x", pos.0.x);
            full_nbt.put_int("y", pos.0.y);
            full_nbt.put_int("z", pos.0.z);
            self.add_block_entity_nbt(block_pos, &full_nbt);
        }
        self.level.read_chunk_sync(&chunk_pos, |chunk| {
            chunk.mark_dirty(true);
        });
    }

    #[must_use]
    pub fn intersects_aabb_with_hit(
        from: Vector3<f64>,
        to: Vector3<f64>,
        min: Vector3<f64>,
        max: Vector3<f64>,
    ) -> Option<(f64, BlockDirection, Vector3<f64>)> {
        let dir = to.sub(&from);
        let mut tmin: f64 = 0.0;
        let mut tmax: f64 = 1.0;

        let mut hit_axis = None;
        let mut hit_is_min = false;

        macro_rules! check_axis {
            ($axis:ident, $dir_axis:ident, $min_axis:ident, $max_axis:ident) => {{
                if dir.$dir_axis.abs() < 1e-8 {
                    if from.$dir_axis < min.$min_axis || from.$dir_axis > max.$max_axis {
                        return None;
                    }
                } else {
                    let inv_d = 1.0 / dir.$dir_axis;
                    let t_near = (min.$min_axis - from.$dir_axis) * inv_d;
                    let t_far = (max.$max_axis - from.$dir_axis) * inv_d;

                    let (t_entry, t_exit, is_min_face) = if inv_d >= 0.0 {
                        (t_near, t_far, true)
                    } else {
                        (t_far, t_near, false)
                    };

                    if t_entry > tmin {
                        tmin = t_entry;
                        hit_axis = Some(stringify!($axis));
                        hit_is_min = is_min_face;
                    }
                    tmax = tmax.min(t_exit);
                    if tmax < tmin {
                        return None;
                    }
                }
            }};
        }

        check_axis!(x, x, x, x);
        check_axis!(y, y, y, y);
        check_axis!(z, z, z, z);

        if tmax <= 1e-7 || tmin > 1.0 {
            return None;
        }

        let direction = match (hit_axis, hit_is_min) {
            (Some("x"), true) => BlockDirection::West,
            (Some("x"), false) => BlockDirection::East,
            (Some("y"), true) => BlockDirection::Down,
            (Some("y"), false) => BlockDirection::Up,
            (Some("z"), true) => BlockDirection::North,
            (Some("z"), false) => BlockDirection::South,
            _ => {
                if dir.y < 0.0 {
                    BlockDirection::Up
                } else if dir.y > 0.0 {
                    BlockDirection::Down
                } else {
                    BlockDirection::North
                }
            }
        };

        let t_hit = tmin.max(0.0);
        let hit_pos = from + dir * t_hit;
        Some((t_hit, direction, hit_pos))
    }

    pub fn ray_outline_check_detailed(
        &self,
        block_pos: &BlockPos,
        from: Vector3<f64>,
        to: Vector3<f64>,
    ) -> Option<(BlockDirection, Vector3<f64>)> {
        let state = self.get_block_state(block_pos);

        if state.outline_shapes.is_empty() && !state.is_waterlogged() {
            return None;
        }

        let bounding_boxes = state.get_block_outline_shapes_at(block_pos);
        let mut closest_hit: Option<(f64, BlockDirection, Vector3<f64>)> = None;

        for shape in bounding_boxes {
            let world_min = shape.min.add(&block_pos.0.to_f64());
            let world_max = shape.max.add(&block_pos.0.to_f64());

            if let Some((t, dir, hit_pos)) =
                Self::intersects_aabb_with_hit(from, to, world_min, world_max)
                && closest_hit
                    .as_ref()
                    .is_none_or(|(closest_t, _, _)| t < *closest_t)
            {
                closest_hit = Some((t, dir, hit_pos));
            }
        }

        closest_hit.map(|(_, dir, hit_pos)| (dir, hit_pos))
    }

    fn ray_outline_check(
        &self,
        block_pos: &BlockPos,
        from: Vector3<f64>,
        to: Vector3<f64>,
    ) -> (bool, Option<BlockDirection>) {
        if let Some((dir, _)) = self.ray_outline_check_detailed(block_pos, from, to) {
            (true, Some(dir))
        } else {
            (false, None)
        }
    }

    #[allow(clippy::too_many_lines)]
    pub fn ray_trace_block(
        &self,
        start_pos: Vector3<f64>,
        end_pos: Vector3<f64>,
        include_fluids: bool,
    ) -> Option<(BlockPos, BlockDirection, Vector3<f64>)> {
        if start_pos == end_pos {
            return None;
        }

        let adjust = -1.0e-7f64;
        let to = end_pos.lerp(&start_pos, adjust);
        let from = start_pos.lerp(&end_pos, adjust);

        let mut block = BlockPos::floored(from.x, from.y, from.z);

        let state = self.get_block_state(&block);
        let valid_start = if include_fluids {
            !state.is_air()
        } else {
            !state.is_air() && !state.is_liquid()
        };
        if valid_start
            && let Some((dir, hit_pos)) = self.ray_outline_check_detailed(&block, from, to)
        {
            return Some((block, dir, hit_pos));
        }

        let difference = to.sub(&from);
        let step = difference.sign();

        let delta = Vector3::new(
            if step.x == 0 {
                f64::MAX
            } else {
                (f64::from(step.x)) / difference.x
            },
            if step.y == 0 {
                f64::MAX
            } else {
                (f64::from(step.y)) / difference.y
            },
            if step.z == 0 {
                f64::MAX
            } else {
                (f64::from(step.z)) / difference.z
            },
        );

        let mut next = Vector3::new(
            delta.x
                * (if step.x > 0 {
                    1.0 - (from.x - from.x.floor())
                } else {
                    from.x - from.x.floor()
                }),
            delta.y
                * (if step.y > 0 {
                    1.0 - (from.y - from.y.floor())
                } else {
                    from.y - from.y.floor()
                }),
            delta.z
                * (if step.z > 0 {
                    1.0 - (from.z - from.z.floor())
                } else {
                    from.z - from.z.floor()
                }),
        );

        while next.x <= 1.0 || next.y <= 1.0 || next.z <= 1.0 {
            let block_direction = match (next.x, next.y, next.z) {
                (x, y, z) if x < y && x < z => {
                    block.0.x += step.x;
                    next.x += delta.x;
                    if step.x > 0 {
                        BlockDirection::West
                    } else {
                        BlockDirection::East
                    }
                }
                (_, y, z) if y < z => {
                    block.0.y += step.y;
                    next.y += delta.y;
                    if step.y > 0 {
                        BlockDirection::Down
                    } else {
                        BlockDirection::Up
                    }
                }
                _ => {
                    block.0.z += step.z;
                    next.z += delta.z;
                    if step.z > 0 {
                        BlockDirection::North
                    } else {
                        BlockDirection::South
                    }
                }
            };

            let state = self.get_block_state(&block);
            let hit = if include_fluids {
                !state.is_air()
            } else {
                !state.is_air() && !state.is_liquid()
            };

            if hit {
                if let Some((dir, hit_pos)) = self.ray_outline_check_detailed(&block, from, to) {
                    return Some((block, dir, hit_pos));
                }
                let block_min = block.0.to_f64();
                let block_max = block_min.add_raw(1.0, 1.0, 1.0);
                if let Some((_, dir, hit_pos)) =
                    Self::intersects_aabb_with_hit(from, to, block_min, block_max)
                {
                    return Some((block, dir, hit_pos));
                }
                return Some((block, block_direction, to));
            }
        }

        None
    }

    pub fn ray_trace_entities(
        &self,
        start: Vector3<f64>,
        end: Vector3<f64>,
    ) -> Vec<(Arc<dyn EntityBase>, Vector3<f64>, f64)> {
        if start == end {
            return Vec::new();
        }

        let min_x = start.x.min(end.x) - 1.0;
        let max_x = start.x.max(end.x) + 1.0;
        let min_y = start.y.min(end.y) - 1.0;
        let max_y = start.y.max(end.y) + 1.0;
        let min_z = start.z.min(end.z) - 1.0;
        let max_z = start.z.max(end.z) + 1.0;
        let ray_box = BoundingBox::new(
            Vector3::new(min_x, min_y, min_z),
            Vector3::new(max_x, max_y, max_z),
        );

        let mut hits = Vec::new();

        for entity in self.entities.load().iter() {
            let bb = entity.get_entity().bounding_box.load();
            if bb.intersects(&ray_box)
                && let Some((t, _, hit_pos)) =
                    Self::intersects_aabb_with_hit(start, end, bb.min, bb.max)
            {
                let distance = (hit_pos - start).length();
                hits.push((entity.clone(), hit_pos, distance, t));
            }
        }

        for player in self.players.load().iter() {
            let bb = player.get_entity().bounding_box.load();
            if bb.intersects(&ray_box)
                && let Some((t, _, hit_pos)) =
                    Self::intersects_aabb_with_hit(start, end, bb.min, bb.max)
            {
                let distance = (hit_pos - start).length();
                hits.push((player.clone() as Arc<dyn EntityBase>, hit_pos, distance, t));
            }
        }

        hits.sort_by(|a, b| a.3.partial_cmp(&b.3).unwrap_or(std::cmp::Ordering::Equal));
        hits.into_iter()
            .map(|(ent, hit_pos, dist, _)| (ent, hit_pos, dist))
            .collect()
    }

    pub fn ray_trace_entity(
        &self,
        start: Vector3<f64>,
        end: Vector3<f64>,
    ) -> Option<(Arc<dyn EntityBase>, Vector3<f64>, f64)> {
        self.ray_trace_entities(start, end).into_iter().next()
    }

    pub fn raycast(
        self: &Arc<Self>,
        start_pos: Vector3<f64>,
        end_pos: Vector3<f64>,
        hit_check: impl Fn(&BlockPos, &Arc<Self>) -> bool,
    ) -> Option<(BlockPos, BlockDirection)> {
        if start_pos == end_pos {
            return None;
        }

        let adjust = -1.0e-7f64;
        let to = end_pos.lerp(&start_pos, adjust);
        let from = start_pos.lerp(&end_pos, adjust);

        let mut block = BlockPos::floored(start_pos.x, start_pos.y, start_pos.z);

        if hit_check(&block, self) {
            let (collision, direction) = self.ray_outline_check(&block, start_pos, end_pos);
            if let Some(dir) = direction
                && collision
            {
                return Some((block, dir));
            }
        }

        let difference = to.sub(&from);

        let step = difference.sign();

        let delta = Vector3::new(
            if step.x == 0 {
                f64::MAX
            } else {
                (f64::from(step.x)) / difference.x
            },
            if step.y == 0 {
                f64::MAX
            } else {
                (f64::from(step.y)) / difference.y
            },
            if step.z == 0 {
                f64::MAX
            } else {
                (f64::from(step.z)) / difference.z
            },
        );

        let mut next = Vector3::new(
            delta.x
                * (if step.x > 0 {
                    1.0 - (from.x - from.x.floor())
                } else {
                    from.x - from.x.floor()
                }),
            delta.y
                * (if step.y > 0 {
                    1.0 - (from.y - from.y.floor())
                } else {
                    from.y - from.y.floor()
                }),
            delta.z
                * (if step.z > 0 {
                    1.0 - (from.z - from.z.floor())
                } else {
                    from.z - from.z.floor()
                }),
        );

        while next.x <= 1.0 || next.y <= 1.0 || next.z <= 1.0 {
            let block_direction = match (next.x, next.y, next.z) {
                (x, y, z) if x < y && x < z => {
                    block.0.x += step.x;
                    next.x += delta.x;
                    if step.x > 0 {
                        BlockDirection::West
                    } else {
                        BlockDirection::East
                    }
                }
                (_, y, z) if y < z => {
                    block.0.y += step.y;
                    next.y += delta.y;
                    if step.y > 0 {
                        BlockDirection::Down
                    } else {
                        BlockDirection::Up
                    }
                }
                _ => {
                    block.0.z += step.z;
                    next.z += delta.z;
                    if step.z > 0 {
                        BlockDirection::North
                    } else {
                        BlockDirection::South
                    }
                }
            };

            if hit_check(&block, self) {
                let (collision, direction) = self.ray_outline_check(&block, start_pos, end_pos);
                if collision {
                    if let Some(dir) = direction {
                        return Some((block, dir));
                    }
                    return Some((block, block_direction));
                }
            }
        }

        None
    }

    pub fn emit_game_event(&self, event_key: impl Into<String>, position: Vector3<f64>) {
        let mut event = crate::plugin::api::events::world::generic_game::GenericGameEvent::new(
            event_key.into(),
            position,
        );
        if let Some(server) = self.server.upgrade() {
            server.plugin_manager.fire_blocking(&server, &mut event);
        }
    }

    pub async fn unload(self: &Arc<Self>) {
        let mut event =
            crate::plugin::api::events::world::world_load::WorldUnloadEvent::new(self.clone());
        if let Some(server) = self.server.upgrade() {
            server.plugin_manager.fire(&server, &mut event).await;
        }
    }

    pub async fn save(&self) {
        for entity in self.entities.load().iter() {
            self.save_entity(entity).await;
        }

        let chunks: Vec<Vector2<i32>> = self
            .block_entities
            .iter()
            .map(|chunk_block_entities| *chunk_block_entities.key())
            .collect();
        for chunk_pos in chunks {
            self.save_block_entities(chunk_pos);
        }

        if let Ok(mut portal_poi) = self.portal_poi.try_lock() {
            let _ = portal_poi.save_all();
        }

        {
            let custom_data = self
                .custom_data
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if !custom_data.is_empty() {
                let custom_data_path = self
                    .level
                    .level_folder
                    .root_folder
                    .join("pumpkin_custom_data.nbt");
                let nbt = pumpkin_nbt::Nbt::from(custom_data.clone());
                let _ = std::fs::write(custom_data_path, nbt.write());
            }
        }

        self.level
            .should_save
            .store(true, std::sync::atomic::Ordering::Relaxed);
        self.level.level_channel.notify();

        let mut save_event = crate::plugin::api::events::world::world_save::WorldSaveEvent::new(
            format!("{:?}", self.dimension),
        );
        if let Some(server) = self.server.upgrade() {
            server.plugin_manager.fire(&server, &mut save_event).await;
        }
    }

    pub fn set_custom_data(&self, namespace: &str, key: &str, value: pumpkin_nbt::tag::NbtTag) {
        let mut custom_data = self
            .custom_data
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        let mut namespace_data = custom_data
            .child_tags
            .remove(namespace)
            .and_then(|tag| match tag {
                pumpkin_nbt::tag::NbtTag::Compound(compound) => Some(compound),
                _ => None,
            })
            .unwrap_or_default();

        namespace_data.child_tags.insert(key.into(), value);
        custom_data.child_tags.insert(
            namespace.into(),
            pumpkin_nbt::tag::NbtTag::Compound(namespace_data),
        );
    }

    pub fn get_custom_data(&self, namespace: &str, key: &str) -> Option<pumpkin_nbt::tag::NbtTag> {
        let custom_data = self
            .custom_data
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        custom_data
            .get(namespace)?
            .extract_compound()?
            .get(key)
            .cloned()
    }

    pub fn remove_custom_data(&self, namespace: &str, key: &str) {
        let mut custom_data = self
            .custom_data
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        let Some(pumpkin_nbt::tag::NbtTag::Compound(mut namespace_data)) =
            custom_data.child_tags.remove(namespace)
        else {
            return;
        };

        namespace_data.child_tags.remove(key);
        if !namespace_data.is_empty() {
            custom_data.child_tags.insert(
                namespace.into(),
                pumpkin_nbt::tag::NbtTag::Compound(namespace_data),
            );
        }
    }

    pub fn has_custom_data(&self, namespace: &str, key: &str) -> bool {
        self.get_custom_data(namespace, key).is_some()
    }

    pub fn set_block_entity_custom_data(
        &self,
        pos: &BlockPos,
        namespace: &str,
        key: &str,
        value: pumpkin_nbt::tag::NbtTag,
    ) {
        let mut entry = self.custom_block_entity_data.entry(*pos).or_default();
        let mut namespace_data = entry
            .child_tags
            .remove(namespace)
            .and_then(|tag| match tag {
                pumpkin_nbt::tag::NbtTag::Compound(compound) => Some(compound),
                _ => None,
            })
            .unwrap_or_default();

        namespace_data.child_tags.insert(key.into(), value);
        entry.child_tags.insert(
            namespace.into(),
            pumpkin_nbt::tag::NbtTag::Compound(namespace_data),
        );
    }

    pub fn get_block_entity_custom_data(
        &self,
        pos: &BlockPos,
        namespace: &str,
        key: &str,
    ) -> Option<pumpkin_nbt::tag::NbtTag> {
        self.custom_block_entity_data
            .get(pos)?
            .get(namespace)?
            .extract_compound()?
            .get(key)
            .cloned()
    }

    pub fn remove_block_entity_custom_data(&self, pos: &BlockPos, namespace: &str, key: &str) {
        if let Some(mut entry) = self.custom_block_entity_data.get_mut(pos) {
            let Some(pumpkin_nbt::tag::NbtTag::Compound(mut namespace_data)) =
                entry.child_tags.remove(namespace)
            else {
                return;
            };

            namespace_data.child_tags.remove(key);
            if !namespace_data.is_empty() {
                entry.child_tags.insert(
                    namespace.into(),
                    pumpkin_nbt::tag::NbtTag::Compound(namespace_data),
                );
            }
        }
    }

    pub fn has_block_entity_custom_data(&self, pos: &BlockPos, namespace: &str, key: &str) -> bool {
        self.get_block_entity_custom_data(pos, namespace, key)
            .is_some()
    }

    pub fn populate_chunk(&self, chunk_pos: Vector2<i32>) {
        let mut populate_event =
            crate::plugin::api::events::world::chunk_populate::ChunkPopulateEvent::new(chunk_pos);
        if let Some(server) = self.server.upgrade() {
            server
                .plugin_manager
                .fire_blocking(&server, &mut populate_event);
        }
    }

    pub fn unload_chunk(&self, chunk_pos: Vector2<i32>) {
        let mut unload_event =
            crate::plugin::api::events::world::chunk_unload::ChunkUnloadEvent::new(chunk_pos);
        if let Some(server) = self.server.upgrade() {
            server
                .plugin_manager
                .fire_blocking(&server, &mut unload_event);
        }
    }

    pub fn load_entities(&self, chunk_pos: Vector2<i32>, entity_count: usize) {
        let mut load_event =
            crate::plugin::api::events::world::entities_load::EntitiesLoadEvent::new(
                chunk_pos,
                entity_count,
            );
        if let Some(server) = self.server.upgrade() {
            server
                .plugin_manager
                .fire_blocking(&server, &mut load_event);
        }
    }

    pub fn unload_entities(&self, chunk_pos: Vector2<i32>, entity_count: usize) {
        let mut unload_event =
            crate::plugin::api::events::world::entities_unload::EntitiesUnloadEvent::new(
                chunk_pos,
                entity_count,
            );
        if let Some(server) = self.server.upgrade() {
            server
                .plugin_manager
                .fire_blocking(&server, &mut unload_event);
        }
    }

    pub fn generate_loot(&self, loot_table: String) {
        let mut loot_event =
            crate::plugin::api::events::world::loot_generate::LootGenerateEvent::new(loot_table);
        if let Some(server) = self.server.upgrade() {
            server
                .plugin_manager
                .fire_blocking(&server, &mut loot_event);
        }
    }

    pub fn skip_time(&self, skip_amount: i64) {
        let mut time_event =
            crate::plugin::api::events::world::time_skip::TimeSkipEvent::new(skip_amount);
        if let Some(server) = self.server.upgrade() {
            server
                .plugin_manager
                .fire_blocking(&server, &mut time_event);
        }
    }

    pub fn trigger_raid(&self, pos: BlockPos) {
        let mut raid_event =
            crate::plugin::api::events::raid::raid_trigger::RaidTriggerEvent::new(pos);
        if let Some(server) = self.server.upgrade() {
            server
                .plugin_manager
                .fire_blocking(&server, &mut raid_event);
        }
    }

    pub fn spawn_raid_wave(&self, wave: u32, pos: BlockPos) {
        let mut wave_event =
            crate::plugin::api::events::raid::raid_spawn_wave::RaidSpawnWaveEvent::new(wave, pos);
        if let Some(server) = self.server.upgrade() {
            server
                .plugin_manager
                .fire_blocking(&server, &mut wave_event);
        }
    }

    pub fn finish_raid(&self, victory: bool) {
        let mut raid_event =
            crate::plugin::api::events::raid::raid_finish::RaidFinishEvent::new(victory);
        if let Some(server) = self.server.upgrade() {
            server
                .plugin_manager
                .fire_blocking(&server, &mut raid_event);
        }
    }

    pub fn stop_raid(&self, reason: String) {
        let mut raid_event =
            crate::plugin::api::events::raid::raid_stop::RaidStopEvent::new(reason);
        if let Some(server) = self.server.upgrade() {
            server
                .plugin_manager
                .fire_blocking(&server, &mut raid_event);
        }
    }

    pub fn async_structure_generate(
        &self,
        world_name: String,
        structure_name: String,
        pos: BlockPos,
    ) {
        let mut event = crate::plugin::api::events::world::async_structure_generate::AsyncStructureGenerateEvent::new(
            world_name,
            structure_name,
            pos,
        );
        if let Some(server) = self.server.upgrade() {
            server.plugin_manager.fire_blocking(&server, &mut event);
        }
    }

    pub fn async_structure_spawn(&self, world_name: String, structure_name: String, pos: BlockPos) {
        let mut event =
            crate::plugin::api::events::world::async_structure_spawn::AsyncStructureSpawnEvent::new(
                world_name,
                structure_name,
                pos,
            );
        if let Some(server) = self.server.upgrade() {
            server.plugin_manager.fire_blocking(&server, &mut event);
        }
    }
}

impl BlockAccessor for World {
    fn get_block(&self, position: &BlockPos) -> &'static Block {
        self.get_block_state_id_if_loaded(position)
            .map_or(&Block::AIR, Block::from_state_id)
    }
    fn get_block_state(&self, position: &BlockPos) -> &'static BlockState {
        self.get_block_state_id_if_loaded(position)
            .map_or(Block::AIR.default_state, BlockState::from_id)
    }

    fn get_block_state_id(&self, position: &BlockPos) -> BlockStateId {
        self.get_block_state_id_if_loaded(position)
            .unwrap_or(Block::AIR.default_state.id)
    }

    fn get_block_and_state(&self, position: &BlockPos) -> (&'static Block, &'static BlockState) {
        let id = self
            .get_block_state_id_if_loaded(position)
            .unwrap_or(Block::AIR.default_state.id);
        BlockState::from_id_with_block(id)
    }
}

pub struct WorldPortal(pub Arc<World>);

// Pure Beauty :cap:
impl WorldPortalExt for WorldPortal {
    fn can_place_at(
        &self,
        block: &pumpkin_data::Block,
        state: &BlockState,
        block_accessor: &dyn BlockAccessor,
        block_pos: &BlockPos,
    ) -> bool {
        self.0.block_registry.can_place_at(
            None,
            None,
            block_accessor,
            None,
            block,
            state,
            block_pos,
            None,
            None,
        )
    }

    fn mirror(&self, block: &Block, state_id: BlockStateId, mirror: Mirror) -> &'static BlockState {
        self.0.block_registry.mirror(block, state_id, mirror)
    }

    fn rotate(
        &self,
        block: &Block,
        state_id: BlockStateId,
        rotation: Rotation,
    ) -> &'static BlockState {
        self.0.block_registry.rotate(block, state_id, rotation)
    }

    fn spawn_mobs_for_chunk_generation(
        &self,
        cache: &mut dyn GenerationCache,
        biome: &'static Biome,
        chunk_x: i32,
        chunk_z: i32,
    ) {
        natural_spawner::spawn_mobs_for_chunk_generation(&self.0, cache, biome, chunk_x, chunk_z);
    }

    fn spawn_structure_entities(&self, entities: Vec<NbtCompound>) {
        for nbt in entities {
            let Some(id) = nbt.get_string("id") else {
                continue;
            };
            let Some(entity_type) =
                EntityType::from_name(id.strip_prefix("minecraft:").unwrap_or(id))
            else {
                warn!("Unknown structure entity type: {id}");
                continue;
            };
            let entity = from_type(
                entity_type,
                Vector3::new(0.0, 0.0, 0.0),
                &self.0,
                Uuid::new_v4(),
            );
            entity.get_entity().read_nbt_non_mut(&nbt);
            entity.read_nbt_non_mut(&nbt);
            self.0.spawn_entity(entity);
        }
    }
}


#[cfg(test)]
mod tests {
    use pumpkin_data::{
        Block,
        block_properties::{ChestLikeProperties, ChestType, HorizontalFacing},
    };
    use pumpkin_util::math::position::BlockPos;

    use crate::world::block_updates::bedrock_block_breaking_rate;
    use super::bedrock_chest_block_actor;

    #[test]
    fn bedrock_block_breaking_rate_uses_progress_per_tick() {
        assert_eq!(bedrock_block_breaking_rate(0.0), 0);
        assert_eq!(bedrock_block_breaking_rate(1.0 / 30.0), 2_184);
        assert_eq!(bedrock_block_breaking_rate(1.0), 65_535);
    }

    #[test]
    fn bedrock_double_chest_block_actor_identifies_pair_and_lead() {
        let position = BlockPos::new(5, 64, 7);
        let properties = ChestLikeProperties {
            facing: HorizontalFacing::North,
            r#type: ChestType::Right,
            waterlogged: false,
        };
        let actor =
            bedrock_chest_block_actor(properties.to_state_id(&Block::CHEST), position).unwrap();

        assert_eq!(actor.get_int("pairx"), Some(4));
        assert_eq!(actor.get_int("pairz"), Some(7));
        assert_eq!(actor.get_bool("pairlead"), Some(true));
    }

    #[test]
    fn game_rules_registry() {
        use pumpkin_data::game_rules::{GameRule, GameRuleRegistry, GameRuleValue};

        let mut registry = GameRuleRegistry::default();
        match registry.get(&GameRule::KeepInventory) {
            GameRuleValue::Bool(v) => assert!(!v),
            GameRuleValue::Int(_) => panic!("expected bool"),
        }

        match registry.get_mut(&GameRule::KeepInventory) {
            GameRuleValue::Bool(v) => *v = true,
            GameRuleValue::Int(_) => panic!("expected bool"),
        }

        match registry.get(&GameRule::KeepInventory) {
            GameRuleValue::Bool(v) => assert!(v),
            GameRuleValue::Int(_) => panic!("expected bool"),
        }

        match registry.get(&GameRule::RandomTickSpeed) {
            GameRuleValue::Int(v) => assert_eq!(*v, 3),
            GameRuleValue::Bool(_) => panic!("expected int"),
        }

        match registry.get_mut(&GameRule::RandomTickSpeed) {
            GameRuleValue::Int(v) => *v = 20,
            GameRuleValue::Bool(_) => panic!("expected int"),
        }

        match registry.get(&GameRule::RandomTickSpeed) {
            GameRuleValue::Int(v) => assert_eq!(*v, 20),
            GameRuleValue::Bool(_) => panic!("expected int"),
        }
    }
}
