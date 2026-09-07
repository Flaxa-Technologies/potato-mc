use std::{collections::HashMap, sync::Arc};

use pumpkin_util::math::{boundingbox::BoundingBox, position::BlockPos, vector3::Vector3};

use crate::entity::{EntityBase, RemovalReason, player::Player};
use crate::net::ClientPlatform;
use crate::world::chunker::{get_view_distance, is_within_view_distance};
use pumpkin_data::entity::EntityType;

use super::World;

impl World {
    /// Gets a `Player` by an entity id
    pub fn get_player_by_id(&self, id: i32) -> Option<Arc<Player>> {
        for player in self.players.load().iter() {
            if player.entity_id() == id {
                return Some(player.clone());
            }
        }
        None
    }

    /// Gets an entity by an entity id
    pub fn get_entity_by_id(&self, id: i32) -> Option<Arc<dyn EntityBase>> {
        for entity in self.entities.load().iter() {
            if entity.get_entity().entity_id == id {
                return Some(entity.clone());
            }
        }
        for player in self.players.load().iter() {
            if player.get_entity().entity_id == id {
                return Some(player.clone() as Arc<dyn EntityBase>);
            }
        }
        None
    }

    /// Gets a `Player` by a username
    pub fn get_player_by_name(&self, name: &str) -> Option<Arc<Player>> {
        for player in self.players.load().iter() {
            if player.gameprofile.name.eq_ignore_ascii_case(name) {
                return Some(player.clone());
            }
        }
        None
    }

    // Gets all entities at a Box
    pub fn get_all_at_box(&self, aabb: &BoundingBox) -> Vec<Arc<dyn EntityBase>> {
        let entities_guard = self.entities.load();
        let players_guard = self.players.load();

        entities_guard
            .iter()
            .map(|e| e.clone() as Arc<dyn EntityBase>)
            .chain(
                players_guard
                    .iter()
                    .map(|p| p.clone() as Arc<dyn EntityBase>),
            )
            .filter(|entity| entity.get_entity().bounding_box.load().intersects(aabb))
            .collect()
    }

    // Gets all non Player entities at a Box
    pub fn get_entities_at_box(&self, aabb: &BoundingBox) -> Vec<Arc<dyn EntityBase>> {
        self.entities
            .load()
            .iter()
            .filter(|entity| entity.get_entity().bounding_box.load().intersects(aabb))
            .cloned()
            .collect()
    }

    // Gets all Player entities at a Box
    pub fn get_players_at_box(&self, aabb: &BoundingBox) -> Vec<Arc<Player>> {
        let players_guard = self.players.load();
        players_guard
            .iter()
            .filter(|player| player.get_entity().bounding_box.load().intersects(aabb))
            .cloned()
            .collect()
    }

    /// Retrieves a player by their unique UUID.
    ///
    /// This function searches the world's active player list for a player with the specified UUID.
    /// If found, it returns an `Arc<Player>` reference to the player. Otherwise, it returns `None`.
    ///
    /// # Arguments
    ///
    /// * `id`: The UUID of the player to retrieve.
    ///
    /// # Returns
    ///
    /// An `Option<Arc<Player>>` containing the player if found, or `None` if not.
    pub fn get_player_by_uuid(&self, id: uuid::Uuid) -> Option<Arc<Player>> {
        self.players
            .load()
            .iter()
            .find(|p| p.gameprofile.id == id)
            .cloned()
    }

    /// Retrieves an entity by their unique UUID.
    ///
    /// This function searches the world's entities for one with the specified UUID.
    /// If found, it returns an `Arc<dyn EntityBase>` reference to that entity. Otherwise, it returns `None`.
    ///
    /// # Arguments
    ///
    /// * `id`: The UUID of the entity to retrieve.
    ///
    /// # Returns
    ///
    /// An `Option<Arc<dyn EntityBase>>` containing the player if found, or `None` if not.
    pub fn get_entity_by_uuid(&self, id: uuid::Uuid) -> Option<Arc<dyn EntityBase>> {
        self.entities
            .load()
            .iter()
            .find(|p| p.get_entity().entity_uuid == id)
            .cloned()
    }

    /// Gets a list of players whose location equals the given position in the world.
    ///
    /// It iterates through the players in the world and checks their location. If the player's location matches the
    /// given position, it will add this to a `Vec` which it later returns. If no
    /// player was found in that position, it will just return an empty `Vec`.
    ///
    /// # Arguments
    ///
    /// * `position`: The position the function will check.
    pub fn get_players_by_pos(&self, position: BlockPos) -> Vec<Arc<Player>> {
        self.players
            .load()
            .iter()
            .filter_map(|player| {
                let player_block_pos = player.get_entity().block_pos.load().0;
                (position.0.x == player_block_pos.x
                    && position.0.y == player_block_pos.y
                    && position.0.z == player_block_pos.z)
                    .then(|| Arc::clone(player))
            })
            .collect::<_>()
    }

    /// Gets the nearby players around a given world position.
    /// It "creates" a sphere and checks if whether players are inside
    /// and returns a `HashMap` where the UUID is the key and the `Player`
    /// object is the value.
    ///
    /// # Arguments
    /// * `pos`: The center of the sphere.
    /// * `radius`: The radius of the sphere. The higher the radius, the more area will be checked (in every direction).
    pub fn get_nearby_players(&self, pos: Vector3<f64>, radius: f64) -> Vec<Arc<Player>> {
        let radius_squared = radius.powi(2);

        self.players
            .load()
            .iter()
            .filter_map(|player| {
                let player_pos = player.get_entity().pos.load();
                (player_pos.squared_distance_to_vec(&pos) <= radius_squared).then(|| player.clone())
            })
            .collect()
    }

    pub fn get_nearby_entities(
        &self,
        pos: Vector3<f64>,
        radius: f64,
    ) -> HashMap<uuid::Uuid, Arc<dyn EntityBase>> {
        let radius_squared = radius.powi(2);

        self.entities
            .load()
            .iter()
            .filter_map(|entity| {
                let entity_pos = entity.get_entity().pos.load();
                (entity_pos.squared_distance_to_vec(&pos) <= radius_squared)
                    .then(|| (entity.get_entity().entity_uuid, entity.clone()))
            })
            .collect()
    }

    pub fn get_closest_player(&self, pos: Vector3<f64>, radius: f64) -> Option<Arc<Player>> {
        let players = self.get_nearby_players(pos, radius);
        players
            .iter()
            .min_by(|a, b| {
                a.get_entity()
                    .pos
                    .load()
                    .squared_distance_to_vec(&pos)
                    .total_cmp(&b.get_entity().pos.load().squared_distance_to_vec(&pos))
            })
            .cloned()
    }

    /// Gets the closest entity to a position, with optional filtering by entity type.
    ///
    /// # Arguments
    ///
    /// * `pos` - The position to search around.
    /// * `radius` - The radius to search within.
    /// * `entity_types` - Optional array of entity types to filter by. If None, all entity types are included.
    ///
    /// # Returns
    ///
    /// The closest entity that matches the filter criteria, or None if no entities are found.
    pub fn get_closest_entity(
        &self,
        pos: Vector3<f64>,
        radius: f64,
        entity_types: Option<&[&'static EntityType]>,
    ) -> Option<Arc<dyn EntityBase>> {
        // Get regular entities
        let entities = self.get_nearby_entities(pos, radius);

        // Filter by entity type if specified
        let filtered_entities = if let Some(types) = entity_types {
            entities
                .into_iter()
                .filter(|(_, entity)| {
                    let entity_type = entity.get_entity().entity_type;
                    types.contains(&entity_type)
                })
                .collect::<HashMap<_, _>>()
        } else {
            entities
        };

        // Find the closest entity
        filtered_entities
            .iter()
            .min_by(|a, b| {
                a.1.get_entity()
                    .pos
                    .load()
                    .squared_distance_to_vec(&pos)
                    .total_cmp(&b.1.get_entity().pos.load().squared_distance_to_vec(&pos))
            })
            .map(|p| p.1.clone())
    }

    /// Adds entities to the provided [`Vec`] that satisfy a particular condition and are
    /// present in the provided [`BoundingBox`].
    ///
    /// # Arguments
    ///
    /// * `list`: The `Vec` to add to.
    /// * `max_list_capacity`: The maximum capacity of `list` for adding entities. If this limit is reached, no more
    ///   entities will be added to the list. If `list` already reaches this limit, nothing happens.
    /// * `bounding_box`: The bounding box to filter any added entities.
    /// * `predicate`: A predicate function, which has to be `true` for an entity to be added to the list.
    pub fn extend_entities_in_box_where(
        &self,
        list: &mut Vec<Arc<dyn EntityBase>>,
        max_list_capacity: usize,
        bounding_box: BoundingBox,
        predicate: impl Fn(&dyn EntityBase) -> bool,
    ) {
        self.extend_entities_where(list, max_list_capacity, |e| {
            bounding_box.intersects(&e.get_entity().bounding_box.load()) && predicate(e)
        });
    }

    /// Adds entities to the provided [`Vec`] that satisfy a particular condition.
    ///
    /// # Arguments
    ///
    /// * `list`: The `Vec` to add to.
    /// * `max_list_capacity`: The maximum capacity of `list` for adding entities. If this limit is reached, no more
    ///   entities will be added to the list. If `list` already reaches this limit, nothing happens.
    /// * `predicate`: A predicate function, which has to be `true` for an entity to be added to the list.
    pub fn extend_entities_where(
        &self,
        list: &mut Vec<Arc<dyn EntityBase>>,
        max_list_capacity: usize,
        predicate: impl Fn(&dyn EntityBase) -> bool,
    ) {
        if list.len() >= max_list_capacity {
            return;
        }
        // Loop the players.
        for player in self.players.load().iter() {
            if !predicate(player.as_ref()) {
                continue;
            }
            // We add the player to the list.
            list.push(player.clone());
            // Check if the list is too big.
            if list.len() > max_list_capacity {
                return;
            }
        }
        // Same with entities.
        for entity in self.entities.load().iter() {
            if !predicate(entity.as_ref()) {
                continue;
            }
            list.push(entity.clone());
            if list.len() > max_list_capacity {
                return;
            }
            // TODO: Implement ender dragon handling
        }
    }

    /// Adds a player to the world and broadcasts a join message if enabled.
    ///
    /// This function takes a player's UUID and an `Arc<Player>` reference.
    /// It inserts the player into the world's `current_players` map using the UUID as the key.
    /// Additionally, it broadcasts a join message to all connected players in the world.
    ///
    /// # Arguments
    ///
    /// * `player`: An `Arc<Player>` reference to the player object.
    pub fn add_player(&self, player: &Arc<Player>) -> Result<(), String> {
        self.players.rcu(|current_list| {
            let mut new_list = (**current_list).clone();
            new_list.push(player.clone());
            new_list
        });
        self.entity_tracker
            .add_entity(&(player.clone() as Arc<dyn EntityBase>), self);
        Ok(())
    }

    /// Must only be called after the player's own `CLogin` packet has been sent.
    pub fn pair_new_player_with_tracked_entities(&self, player: &Arc<Player>) {
        self.entity_tracker
            .pair_new_player_with_tracked_entities(player, self);
    }

    /// Removes a player from the world and broadcasts a disconnect message if enabled.
    ///
    /// This function removes a player from the world based on their `Player` reference.
    /// It performs the following actions:
    ///
    /// 1. Removes the player from the `current_players` map using their UUID.
    /// 2. Broadcasts a `CRemovePlayerInfo` packet to all connected players to inform them about the player leaving.
    /// 3. Removes the player's entity from the world using its entity ID.
    /// 4. Optionally sends a disconnect message to all other players notifying them about the player leaving.
    ///
    /// # Arguments
    ///
    /// * `player`: A reference to the `Player` object to be removed.
    /// * `fire_event`: A boolean flag indicating whether to fire a `PlayerLeaveEvent` event.
    ///
    /// # Notes
    ///
    /// - This function assumes `broadcast_packet_expect` and `remove_entity` are defined elsewhere.
    /// - The disconnect message sending is currently optional. Consider making it a configurable option.
    pub async fn remove_player(
        &self,
        player: &Arc<Player>,
        fire_event: bool,
    ) -> Option<Arc<Player>> {
        use pumpkin_protocol::codec::var_long::VarLong;
        use pumpkin_protocol::bedrock::client::player_list::{
            CPlayerList, PlayerListEntry, Skin,
        };
        use pumpkin_protocol::bedrock::client::common::BuildPlatform;
        use pumpkin_protocol::bedrock::client::remove_actor::CRemoveActor;
        use pumpkin_protocol::java::client::play::{CRemoveEntities, CRemovePlayerInfo};
        use pumpkin_util::text::{TextComponent, color::NamedColor};
        use pumpkin_data::translation;
        use crate::plugin::player::player_leave::PlayerLeaveEvent;
        use tracing::info;

        let mut removed_player: Option<Arc<Player>> = None;

        self.players.rcu(|current_list| {
            let mut new_list = (**current_list).clone();
            // Find the player before we filter them out
            let pos = new_list
                .iter()
                .position(|p| p.gameprofile.id == player.gameprofile.id);
            if let Some(pos) = pos {
                removed_player = Some(new_list.remove(pos));
            }
            new_list
        });
        if let Some(ref player) = removed_player {
            self.entity_tracker
                .remove_entity(player.as_ref() as &dyn EntityBase, self);
            let uuid = player.gameprofile.id;
            let entity_id = player.entity_id();

            let bedrock_remove_player = CPlayerList {
                action: CPlayerList::ACTION_REMOVE,
                entries: vec![PlayerListEntry {
                    uuid,
                    entity_unique_id: VarLong(entity_id as i64),
                    username: player.gameprofile.name.clone(),
                    xuid: String::new(),
                    platform_chat_id: String::new(),
                    build_platform: BuildPlatform::Unknown,
                    skin: Skin::steve(),
                    is_teacher: false,
                    is_host: false,
                    is_sub_client: false,
                    player_color: [0, 0, 0, 0],
                }],
            };

            self.broadcast_editioned(&CRemovePlayerInfo::new(&[uuid]), &bedrock_remove_player);

            self.broadcast_editioned(
                &CRemoveEntities::new(&[entity_id.into()]),
                &CRemoveActor::new(VarLong(entity_id as i64)),
            );

            if fire_event {
                let msg_comp = TextComponent::translate_cross(
                    translation::java::MULTIPLAYER_PLAYER_LEFT,
                    translation::bedrock::MULTIPLAYER_PLAYER_LEFT,
                    [TextComponent::text(player.gameprofile.name.clone())],
                )
                .color_named(NamedColor::Yellow);
                let mut event = PlayerLeaveEvent::new(player.clone(), msg_comp);

                if let Some(server) = self.server.upgrade() {
                    server.plugin_manager.fire(&server, &mut event).await;

                    if !event.cancelled {
                        for player in self.players.load().iter() {
                            player.send_system_message(&event.leave_message);
                        }
                        info!("{}", event.leave_message.to_pretty_console());
                    }
                }
            }
        }
        removed_player
    }

    #[expect(clippy::needless_pass_by_value)]
    pub fn spawn_entity_non_save(&self, entity: Arc<dyn EntityBase>) {
        let _base_entity = entity.get_entity();
        entity.init_data_tracker();
        self.entity_tracker.add_entity(&entity, self);
        self.spawn_state.load().add_entity(self, entity.as_ref());

        self.entities.rcu(|current_entities| {
            let mut new_entities = (**current_entities).clone();
            new_entities.push(entity.clone());
            new_entities
        });
    }

    pub fn spawn_entity(self: &Arc<Self>, entity: Arc<dyn EntityBase>) {
        let mut event = crate::plugin::api::events::entity::entity_spawn::EntitySpawnEvent::new(
            entity.get_entity().entity_id,
            entity.get_entity().entity_type.id.to_string(),
            entity.get_entity().pos.load(),
            self.clone(),
        );
        if let Some(server) = self.server.upgrade() {
            server.plugin_manager.fire_blocking(&server, &mut event);
        }
        if event.cancelled {
            return;
        }

        entity.init_data_tracker();
        self.add_entity_silent(entity);
    }

    pub fn broadcast_entity_spawn(&self, entity: &Arc<dyn EntityBase>) {
        let base_entity = entity.get_entity();
        let chunk_pos = base_entity.chunk_pos.load();

        let players = self.players.load();
        for player in players.iter() {
            let center = player.get_entity().chunk_pos.load();
            let view_distance = get_view_distance(player).get() as i32;

            if is_within_view_distance(chunk_pos, center, view_distance) {
                player.client.try_enqueue_spawn_packet(entity);
            }
        }
    }

    #[expect(clippy::needless_pass_by_value)]
    pub fn add_entity_silent(&self, entity: Arc<dyn EntityBase>) {
        let base_entity = entity.get_entity();

        // Guard against duplicate entities with the same UUID.
        // This can happen when chunk entity data is loaded while the entity
        // already exists in the world (e.g. another player is still tracking it).
        let already_exists = self
            .entities
            .load()
            .iter()
            .any(|e| e.get_entity().entity_uuid == base_entity.entity_uuid);
        if already_exists {
            return;
        }

        // The entity stays live-only: it is written to its chunk's saved data on
        // unload (see `save_entity`), never at spawn, so it can't be both live and
        // serialized at once (which would double it on the next reload).
        self.spawn_state.load().add_entity(self, entity.as_ref());
        self.entity_tracker.add_entity(&entity, self);

        self.entities.rcu(|current_entities| {
            let mut new_entities = (**current_entities).clone();
            new_entities.push(entity.clone());
            new_entities
        });
    }

    pub fn remove_entity(&self, entity: &dyn EntityBase) {
        use std::sync::atomic::Ordering;
        let base_entity = entity.get_entity();
        if base_entity
            .removal_reason
            .swap(Some(RemovalReason::Discarded))
            .is_some()
        {
            return;
        }
        base_entity.removed.store(true, Ordering::Release);

        self.spawn_state.load().remove_entity(self, entity);
        self.entity_tracker.remove_entity(entity, self);
        self.entities.rcu(|current_entities| {
            let mut new_entities = (**current_entities).clone();
            new_entities.retain(|e| e.get_entity().entity_uuid != base_entity.entity_uuid);
            new_entities
        });
    }

    pub async fn remove_entities_in_chunks(
        &self,
        chunks: impl IntoIterator<Item = impl std::borrow::Borrow<pumpkin_util::math::vector2::Vector2<i32>>>,
    ) {
        use rustc_hash::FxHashSet;
        let chunks_set: FxHashSet<_> = chunks.into_iter().map(|c| *c.borrow()).collect();
        if chunks_set.is_empty() {
            return;
        }
        let mut entities_to_remove = Vec::new();

        self.entities.rcu(|current_entities| {
            let mut new_entities = (**current_entities).clone();
            new_entities.retain(|entity| {
                let base_entity = entity.get_entity();
                let pos = base_entity.chunk_pos.load();
                if chunks_set.contains(&pos) {
                    entities_to_remove.push(entity.clone());
                    false
                } else {
                    true
                }
            });
            new_entities
        });

        for entity in entities_to_remove {
            self.entity_tracker.remove_entity(entity.as_ref(), self);
            self.save_entity(&entity).await;
            self.spawn_state.load().remove_entity(self, entity.as_ref());
        }

        for chunk_pos in &chunks_set {
            self.save_block_entities(*chunk_pos);
            self.block_entities.remove(chunk_pos);
        }
    }
}
