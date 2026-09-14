use std::sync::Arc;

use pumpkin_data::Block;
use pumpkin_data::block_properties::HorizontalAxis;
use pumpkin_data::dimension::Dimension;
use pumpkin_util::math::position::BlockPos;
use pumpkin_util::math::vector2::Vector2;
use pumpkin_util::math::vector3::Vector3;
use pumpkin_world::world::BlockFlags;

use super::World;
use crate::entity::EntityBase;

pub mod end;
pub mod nether;
pub mod poi;

pub use nether::{NetherPortal, PortalSearchResult};
pub use poi::PortalPoiStorage;

#[derive(Clone)]
pub struct SourcePortalInfo {
    pub lower_corner: BlockPos,
    pub axis: HorizontalAxis,
    pub width: u32,
    pub height: u32,
}

impl From<&PortalSearchResult> for SourcePortalInfo {
    fn from(result: &PortalSearchResult) -> Self {
        Self {
            lower_corner: result.lower_corner,
            axis: result.axis,
            width: result.width,
            height: result.height,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PortalType {
    Nether,
    End,
}

fn block_on_task<F, R>(handle: &tokio::runtime::Handle, f: F) -> R
where
    F: std::future::Future<Output = R> + Send + 'static,
    R: Send + 'static,
{
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    handle.spawn(async move {
        let res = f.await;
        let _ = tx.send(res);
    });
    rx.recv().expect("portal async task panicked or channel closed")
}

impl PortalType {
    pub fn get_portal_transition_time(
        &self,
        current_world: &World,
        entity: &dyn crate::entity::EntityBase,
    ) -> u32 {
        match self {
            Self::End => 0,
            Self::Nether => {
                let entity_type = entity.get_entity().entity_type;
                let level_info = current_world.level_info.load();
                match entity_type.id {
                    id if id == pumpkin_data::entity::EntityType::PLAYER.id => (current_world
                        .get_player_by_id(entity.get_entity().entity_id))
                    .map_or(80, |player| match player.gamemode.load() {
                        pumpkin_util::GameMode::Creative => {
                            level_info.game_rules.players_nether_portal_creative_delay as u32
                        }
                        _ => level_info.game_rules.players_nether_portal_default_delay as u32,
                    }),
                    _ => 0,
                }
            }
        }
    }

    #[expect(clippy::too_many_lines)]
    pub fn get_portal_destination(
        &self,
        current_level: &World,
        dest_world: Arc<World>,
        caller: &dyn EntityBase,
        source_portal: Option<&SourcePortalInfo>,
    ) -> Option<TeleportTransition> {
        let is_same_dimension = current_level.dimension == dest_world.dimension;
        match self {
            Self::End => {
                if is_same_dimension {
                    None
                } else {
                    if dest_world.dimension == Dimension::THE_END {
                        // Entering the End: spawn on the obsidian platform at (100, 49, 0) for players, or (100, 50, 0) for other entities
                        let is_player = caller
                            .get_living_entity()
                            .is_some_and(crate::entity::living::LivingEntity::is_player);
                        let y = if is_player { 49.0 } else { 50.0 };

                        let platform_pos = BlockPos::new(100, 49, 0);

                        // Ensure chunks covering the platform are loaded/generated
                        if let Ok(handle) = tokio::runtime::Handle::try_current() {
                            let dest = dest_world.clone();
                            block_on_task(&handle, async move {
                                let center_chunk =
                                    Vector2::new(platform_pos.0.x >> 4, platform_pos.0.z >> 4);
                                dest.level
                                    .get_or_fetch_chunk(center_chunk, |_| ())
                                    .await;
                            });
                        }

                        // Generate/regenerate the obsidian platform (5x5 obsidian at Y=48, and 5x5x3 air above it)
                        for dx in -2..=2 {
                            for dz in -2..=2 {
                                for dy in -1..3 {
                                    let block = if dy == -1 {
                                        Block::OBSIDIAN
                                    } else {
                                        Block::AIR
                                    };
                                    let block_pos =
                                        BlockPos::new(platform_pos.0.x + dx, 49 + dy, platform_pos.0.z + dz);
                                    dest_world.set_block_state(
                                        &block_pos,
                                        block.default_state.id,
                                        BlockFlags::NOTIFY_ALL,
                                    );
                                }
                            }
                        }

                        Some(TeleportTransition {
                            new_world: dest_world,
                            position: Vector3::new(100.5, y, 0.5),
                            yaw: Some(90.0), // Face west toward the main island
                            pitch: Some(0.0),
                        })
                    } else {
                        // Leaving the End: return to spawn/bed in destination world
                        // For players, show the credits/end poem on first exit
                        if let Some(living) = caller.get_living_entity()
                            && let Some(player) = living.get_player()
                        {
                            match player.client.as_ref() {
                                crate::net::ClientPlatform::Java(client) => {
                                    if let Ok(data) = client.serialize_packet(&pumpkin_protocol::java::client::play::CGameEvent::new(
                                        pumpkin_protocol::java::client::play::GameEvent::WinGame,
                                        1.0,
                                    )) {
                                        client.try_enqueue_packet(data);
                                    }
                                }
                                crate::net::ClientPlatform::Bedrock(client) => {
                                    if let Ok(data) = client.serialize_packet(
                                        &pumpkin_protocol::bedrock::client::CShowCredits {
                                            player_runtime_id: (caller.get_entity().entity_id
                                                as u64)
                                                .into(),
                                            credits_state: 0.into(),
                                        },
                                    ) {
                                        client.try_enqueue_packet(data);
                                    }
                                }
                            }
                        }

                        // Teleport to destination world's spawn point
                        let level_info = dest_world.level_info.load();
                        Some(TeleportTransition {
                            new_world: dest_world,
                            position: Vector3::new(
                                f64::from(level_info.spawn_x) + 0.5,
                                f64::from(level_info.spawn_y),
                                f64::from(level_info.spawn_z) + 0.5,
                            ),
                            yaw: Some(level_info.spawn_yaw),
                            pitch: Some(0.0),
                        })
                    }
                }
            }
            Self::Nether => {
                let pos = caller.get_entity().pos.load();
                let current_yaw = caller.get_entity().yaw.load();
                let dimensions = caller.get_entity().entity_dimension.load();
                let coordinate_scale = current_level.dimension.coordinate_scale
                    / dest_world.dimension.coordinate_scale;
                let scaled_x = (pos.x * coordinate_scale).floor() as i32;
                let scaled_z = (pos.z * coordinate_scale).floor() as i32;
                let (clamped_x, clamped_z) = dest_world
                    .worldborder
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .clamp_block(scaled_x, scaled_z);

                let approximate_exit_pos =
                    BlockPos::new(clamped_x, pos.y.floor() as i32, clamped_z);
                let source_portal_axis = source_portal.map_or(HorizontalAxis::X, |p| p.axis);

                // Ensure chunks around approximate_exit_pos are generated/loaded in dest_world
                // before searching, matching Vanilla poiManager.ensureLoadedAndValid(level, exitPos, radius).
                if let Ok(handle) = tokio::runtime::Handle::try_current() {
                    let dest = dest_world.clone();
                    block_on_task(&handle, async move {
                        let center_chunk = Vector2::new(
                            approximate_exit_pos.0.x >> 4,
                            approximate_exit_pos.0.z >> 4,
                        );
                        let chunk_radius = if dest.dimension.has_ceiling {
                            1
                        } else {
                            2
                        };
                        let mut futures = Vec::new();
                        for dx in -chunk_radius..=chunk_radius {
                            for dz in -chunk_radius..=chunk_radius {
                                let chunk_pos =
                                    Vector2::new(center_chunk.x + dx, center_chunk.y + dz);
                                let dest_clone = dest.clone();
                                futures.push(async move {
                                    dest_clone
                                        .level
                                        .get_or_fetch_chunk(chunk_pos, |_| ())
                                        .await;
                                });
                            }
                        }
                        futures::future::join_all(futures).await;
                    });
                }

                let (exit_portal, created_portal_pos) = if let Some(found) = NetherPortal::search_for_portal(
                    &dest_world,
                    approximate_exit_pos,
                ) {
                    (Some(found), None)
                } else if let Some((build_pos, axis, is_fallback)) = NetherPortal::find_safe_location(
                    &dest_world,
                    approximate_exit_pos,
                    source_portal_axis,
                ) {
                    // Ensure chunks covering build_pos are fully loaded and in memory before placing portal frame blocks
                    if let Ok(handle) = tokio::runtime::Handle::try_current() {
                        let dest = dest_world.clone();
                        block_on_task(&handle, async move {
                            let center = Vector2::new(build_pos.0.x >> 4, build_pos.0.z >> 4);
                            let mut futures = Vec::new();
                            for dx in -1..=1 {
                                for dz in -1..=1 {
                                    let chunk_pos = Vector2::new(center.x + dx, center.y + dz);
                                    let dest_clone = dest.clone();
                                    futures.push(async move {
                                        dest_clone.level.get_or_fetch_chunk(chunk_pos, |_| ()).await;
                                    });
                                }
                            }
                            futures::future::join_all(futures).await;
                        });
                    }
                    NetherPortal::build_portal_frame(&dest_world, build_pos, axis, is_fallback);
                    (
                        Some(PortalSearchResult {
                            lower_corner: build_pos,
                            axis,
                            width: 2,
                            height: 3,
                        }),
                        Some(build_pos),
                    )
                } else {
                    (None, None)
                };

                let (final_pos, yaw) = exit_portal.as_ref().map_or_else(
                    || (approximate_exit_pos.0.to_f64(), None),
                    |exit_portal| {
                        let relative_offset = source_portal.map_or_else(
                            || Vector3::new(0.5, 0.0, 0.0),
                            |source| {
                                let source_result = PortalSearchResult {
                                    lower_corner: source.lower_corner,
                                    axis: source.axis,
                                    width: source.width,
                                    height: source.height,
                                };
                                source_result.entity_pos_in_portal(pos, &dimensions)
                            },
                        );
                        let target_pos =
                            exit_portal.calculate_exit_position(relative_offset, &dimensions);
                        let collision_free_pos =
                            exit_portal.find_open_position(&dest_world, target_pos, &dimensions);
                        let yaw = exit_portal
                            .calculate_teleport_yaw(current_yaw, source_portal.map(|p| p.axis));
                        (collision_free_pos, Some(yaw))
                    },
                );

                let direction = if dest_world.dimension == Dimension::THE_NETHER {
                    "OVERWORLD_TO_NETHER"
                } else {
                    "NETHER_TO_OVERWORLD"
                };
                let is_nether_dest = dest_world.dimension == Dimension::THE_NETHER;
                let search_radius = if is_nether_dest {
                    nether::SEARCH_RADIUS_NETHER
                } else {
                    nether::SEARCH_RADIUS_OVERWORLD
                };

                tracing::info!(
                    "[PORTAL] direction={} source_world=\"{}\" source=({:.2},{:.2},{:.2}) scaled_target=({},{},{}) search_radius={} selected_portal={:?} created_portal={:?} destination=({:.2},{:.2},{:.2}) orientation={:?}",
                    direction,
                    current_level.dimension.minecraft_name,
                    pos.x,
                    pos.y,
                    pos.z,
                    clamped_x,
                    pos.y.floor() as i32,
                    clamped_z,
                    search_radius,
                    exit_portal.as_ref().map(|p| p.lower_corner),
                    created_portal_pos,
                    final_pos.x,
                    final_pos.y,
                    final_pos.z,
                    exit_portal.as_ref().map(|p| p.axis),
                );

                Some(TeleportTransition {
                    new_world: dest_world,
                    position: final_pos,
                    yaw,
                    pitch: None,
                })
            }
        }
    }
}

pub struct TeleportTransition {
    pub new_world: Arc<World>,
    pub position: Vector3<f64>,
    pub yaw: Option<f32>,
    pub pitch: Option<f32>,
}

pub struct PortalProcessor {
    pub portal_type: PortalType,
    pub entry_position: BlockPos,
    pub portal_time: u32,
    pub inside_portal_this_tick: bool,
    pub destination_world: Arc<World>,
    pub source_portal: Option<SourcePortalInfo>,
}

impl PortalProcessor {
    pub const fn new(
        portal_type: PortalType,
        entry_position: BlockPos,
        destination_world: Arc<World>,
    ) -> Self {
        Self {
            portal_type,
            entry_position,
            portal_time: 0,
            inside_portal_this_tick: true,
            destination_world,
            source_portal: None,
        }
    }

    pub const fn set_source_portal(&mut self, info: SourcePortalInfo) {
        self.source_portal = Some(info);
    }

    pub fn process_portal_teleportation(
        &mut self,
        current_world: &World,
        entity: &dyn crate::entity::EntityBase,
        allowed_to_teleport: bool,
    ) -> bool {
        if self.inside_portal_this_tick {
            self.inside_portal_this_tick = false;
            if allowed_to_teleport {
                self.portal_time += 1;
                let transition_time = self
                    .portal_type
                    .get_portal_transition_time(current_world, entity);
                self.portal_time >= transition_time
            } else {
                false
            }
        } else {
            self.decay_tick();
            false
        }
    }

    pub const fn decay_tick(&mut self) {
        self.portal_time = self.portal_time.saturating_sub(4);
    }

    #[must_use]
    pub const fn has_expired(&self) -> bool {
        self.portal_time == 0
    }
}
