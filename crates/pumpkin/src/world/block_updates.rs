use std::sync::Arc;

use pumpkin_data::block_properties::is_air;
use pumpkin_data::fluid::Fluid;
use pumpkin_data::item_stack::ItemStack;
use pumpkin_data::tag::Taggable;
use pumpkin_data::world::WorldEvent;
use pumpkin_data::{Block, BlockDirection, BlockState, BlockStateId};
use pumpkin_protocol::bedrock::client::block_event::CBlockEvent as CBedrockBlockEvent;
use pumpkin_protocol::bedrock::client::level_event::{CLevelEvent, LevelEvent};
use pumpkin_protocol::codec::var_int::VarInt;
use pumpkin_protocol::java::client::play::{
    CBlockEvent, CBlockUpdate, CSetBlockDestroyStage, CWorldEvent,
};
use pumpkin_util::math::position::BlockPos;
use pumpkin_util::math::vector3::Vector3;
use pumpkin_util::random::{RandomImpl, get_seed, xoroshiro128::Xoroshiro};
use pumpkin_world::tick::TickPriority;

use crate::block::{BlockEvent, OnNeighborUpdateArgs};
use crate::entity::player::Player;
use crate::entity::{Entity, EntityBase};
use crate::entity::item::ItemEntity;
use crate::net::ClientPlatform;
use crate::world::BlockBreakingProgress;
use crate::world::chunker::{get_view_distance, is_within_view_distance};
use pumpkin_data::entity::EntityType;
use pumpkin_world::inventory::Inventory;
pub use pumpkin_world::world::BlockFlags;

use rand::RngExt;

use super::World;

impl World {
    pub fn add_synced_block_event(&self, pos: BlockPos, r#type: u8, data: u8) {
        let mut queue = self
            .synced_block_event_queue
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        queue.push(BlockEvent { pos, r#type, data });
    }

    pub fn flush_synced_block_events(self: &Arc<Self>) {
        // THIS IS IMPORTANT
        // it prevents deadlocks and also removes the need to wait for a lock when adding a new synced block
        let events = {
            let mut queue = self
                .synced_block_event_queue
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            std::mem::take(&mut *queue)
        };

        for event in events {
            let block = self.get_block(&event.pos);
            if !self.block_registry.on_synced_block_event(
                block,
                self,
                &event.pos,
                event.r#type,
                event.data,
            ) {
                continue;
            }
            let chunk_pos = event.pos.chunk_position();
            self.broadcast_to_chunk_editioned(
                chunk_pos,
                &CBlockEvent::new(
                    event.pos,
                    event.r#type,
                    event.data,
                    VarInt(block.id.as_u16() as i32),
                ),
                &CBedrockBlockEvent {
                    block_position: event.pos,
                    event_type: event.r#type.into(),
                    event_value: event.data.into(),
                },
            );
        }
    }

    pub fn register_block_change(&self, position: BlockPos, block_state_id: BlockStateId) {
        self.unsent_block_changes
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(position, block_state_id);
    }

    /// Queues block state changes for broadcast to nearby players.
    ///
    /// Call [`flush_block_updates`](Self::flush_block_updates) afterward to send the packets.
    pub fn queue_block_updates(&self, changes: &[(BlockPos, BlockStateId)]) {
        let mut guard = self
            .unsent_block_changes
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        for (pos, state_id) in changes {
            guard.insert(*pos, *state_id);
        }
    }

    #[expect(clippy::too_many_lines)]
    pub fn flush_block_updates(&self) {
        use std::collections::HashMap;
        use pumpkin_util::math::position::chunk_section_from_pos;
        use pumpkin_util::math::vector2::Vector2;
        use pumpkin_protocol::java::client::play::{CBlockEntityData, CMultiBlockUpdate};
        use pumpkin_protocol::bedrock::client::block_actor_data::CBlockActorData;
        use pumpkin_world::chunk::palette::bedrock_water_state;

        let mut block_state_updates_by_chunk_section: HashMap<
            pumpkin_util::math::vector3::Vector3<i32>,
            Vec<(BlockPos, BlockStateId)>,
        > = HashMap::new();
        let changes = {
            let mut guard = self
                .unsent_block_changes
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            std::mem::take(&mut *guard)
        };
        for (position, block_state_id) in changes {
            let chunk_section = chunk_section_from_pos(&position);
            block_state_updates_by_chunk_section
                .entry(chunk_section)
                .or_default()
                .push((position, block_state_id));
        }

        // TODO: only send packet to players who have the chunks loaded
        // TODO: Send light updates to update the wire directly next to a broken block
        for (chunk_section, updates) in block_state_updates_by_chunk_section {
            if updates.is_empty() {
                continue;
            }
            let chunk_pos = Vector2::new(chunk_section.x, chunk_section.z);
            if updates.len() == 1 {
                let (block_pos, block_state_id) = updates[0];
                let be_block_id = BlockState::to_be_network_id(block_state_id);
                self.broadcast_to_chunk_editioned(
                    chunk_pos,
                    &CBlockUpdate::new(block_pos, i32::from(block_state_id.as_u16()).into()),
                    &pumpkin_protocol::bedrock::client::CUpdateBlock::new(
                        block_pos,
                        be_block_id as u32,
                    ),
                );
                if let Some(block_entity) = self.get_block_entity(&block_pos)
                    && let Some(nbt) = block_entity.chunk_data_nbt()
                {
                    let bytes = pumpkin_nbt::Nbt::from(nbt).write_unnamed();
                    self.broadcast_to_chunk(
                        chunk_pos,
                        &CBlockEntityData::new(
                            block_pos,
                            VarInt(block_entity.get_id() as i32),
                            bytes.as_ref().into(),
                        ),
                    );
                }
                if let Some(data) = self.bedrock_block_entity_data(block_state_id, block_pos) {
                    self.broadcast_to_chunk_bedrock(
                        chunk_pos,
                        &CBlockActorData::new(block_pos, data),
                    );
                }
            } else {
                let players = self.players.load();
                let mut java_recipients = Vec::new();

                let recipients = players.iter().filter(|p| {
                    let center = p.get_entity().chunk_pos.load();
                    let view_distance = get_view_distance(p).get() as i32;
                    is_within_view_distance(chunk_pos, center, view_distance)
                });

                let mut bedrock_packets = Vec::new();
                for (block_pos, block_state_id) in &updates {
                    let be_block_id = BlockState::to_be_network_id(*block_state_id);
                    let update_packet = pumpkin_protocol::bedrock::client::CUpdateBlock::new(
                        *block_pos,
                        be_block_id as u32,
                    );
                    let actor_packet = self
                        .bedrock_block_entity_data(*block_state_id, *block_pos)
                        .map(|data| CBlockActorData::new(*block_pos, data));
                    bedrock_packets.push((update_packet, actor_packet));
                }

                let mut bedrock_recipients = Vec::new();
                for p in recipients {
                    match p.client.as_ref() {
                        ClientPlatform::Java(_) => java_recipients.push(p),
                        ClientPlatform::Bedrock(be_client) => {
                            bedrock_recipients.push(be_client);
                        }
                    }
                }

                for be_client in bedrock_recipients {
                    for (update_packet, actor_packet) in &bedrock_packets {
                        if let Ok(data) = be_client.serialize_packet(update_packet) {
                            be_client.try_enqueue_packet(data);
                        }
                        if let Some(actor_packet) = actor_packet
                            && let Ok(data) = be_client.serialize_packet(actor_packet)
                        {
                            be_client.try_enqueue_packet(data);
                        }
                    }
                }

                let recipients_by_version =
                    Self::collect_java_recipients_by_version(java_recipients.into_iter());
                Self::broadcast_java_grouped(
                    &CMultiBlockUpdate::new(&updates),
                    recipients_by_version,
                );

                for (block_pos, _) in &updates {
                    if let Some(block_entity) = self.get_block_entity(block_pos)
                        && let Some(nbt) = block_entity.chunk_data_nbt()
                    {
                        let bytes = pumpkin_nbt::Nbt::from(nbt).write_unnamed();
                        self.broadcast_to_chunk(
                            chunk_pos,
                            &CBlockEntityData::new(
                                *block_pos,
                                VarInt(block_entity.get_id() as i32),
                                bytes.as_ref().into(),
                            ),
                        );
                    }
                }
            }

            let mut bedrock_water_packets = Vec::new();
            for (block_pos, block_state_id) in &updates {
                let water_state = bedrock_water_state(*block_state_id);
                let packet = pumpkin_protocol::bedrock::client::CUpdateBlock::with_layer(
                    *block_pos,
                    u32::from(BlockState::to_be_network_id(water_state)),
                    1,
                );
                bedrock_water_packets.push(packet);
            }

            if !bedrock_water_packets.is_empty() {
                let players = self.players.load();
                let recipients = players.iter().filter(|player| {
                    let center = player.get_entity().chunk_pos.load();
                    let view_distance = get_view_distance(player).get() as i32;
                    is_within_view_distance(chunk_pos, center, view_distance)
                });
                for player in recipients {
                    if let ClientPlatform::Bedrock(client) = player.client.as_ref() {
                        for packet in &bedrock_water_packets {
                            if let Ok(data) = client.serialize_packet(packet) {
                                client.try_enqueue_packet(data);
                            }
                        }
                    }
                }
            }
        }
    }

    #[expect(clippy::too_many_lines)]
    pub fn set_block_state(
        self: &Arc<Self>,
        position: &BlockPos,
        block_state_id: BlockStateId,
        flags: BlockFlags,
    ) -> BlockStateId {
        if !self.is_in_build_limit(*position) {
            return Block::AIR.default_state.id;
        }

        let (chunk_coordinate, relative) = position.chunk_and_chunk_relative_position();
        let replaced_block_state_id = self
            .level
            .read_chunk_sync(&chunk_coordinate, |chunk| {
                let replaced_block_state_id = chunk.set_block_absolute_y(
                    relative.x as usize,
                    relative.y,
                    relative.z as usize,
                    block_state_id,
                );
                replaced_block_state_id
            })
            .unwrap_or(Block::AIR.default_state.id);

        if !flags.contains(BlockFlags::FORCE_STATE) && replaced_block_state_id == block_state_id {
            return block_state_id;
        }

        let old_block = Block::from_state_id(replaced_block_state_id);
        let new_block = Block::from_state_id(block_state_id);
        let is_new_block = old_block != new_block;
        let block_moved = flags.contains(BlockFlags::MOVED);

        if is_new_block
            && old_block.default_state.block_entity_type != u16::MAX
            && let Some(entity) = self.get_block_entity(position)
        {
            if !flags.contains(BlockFlags::SKIP_BLOCK_ENTITY_REPLACED_CALLBACK) {
                entity.on_block_replaced(self, position);
            }
            self.remove_block_entity(position);
        }

        if is_new_block && (flags.contains(BlockFlags::NOTIFY_NEIGHBORS) || block_moved) {
            self.block_registry.on_state_replaced(
                self,
                old_block,
                position,
                replaced_block_state_id,
                block_moved,
            );
        }

        if !flags.contains(BlockFlags::SKIP_BLOCK_ADDED_CALLBACK) && is_new_block {
            self.block_registry.on_placed(
                self,
                new_block,
                block_state_id,
                position,
                replaced_block_state_id,
                block_moved,
            );
            let new_fluid = self.get_fluid(position);
            self.block_registry.on_placed_fluid(
                self,
                new_fluid,
                block_state_id,
                position,
                replaced_block_state_id,
                block_moved,
            );
        }

        // Level.java setBlock
        if self.get_block_state_id(position) == block_state_id {
            if flags.contains(BlockFlags::NOTIFY_LISTENERS) {
                self.unsent_block_changes
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .insert(*position, block_state_id);
            }

            if flags.contains(BlockFlags::NOTIFY_NEIGHBORS) {
                self.update_neighbors_at(position, old_block, None);
                if block_state_id.has_analog_output_signal() {
                    self.update_neighbour_for_output_signal(position, new_block);
                }
            }

            if !flags.contains(BlockFlags::MOVED) {
                let mut neighbour_update_flags = flags;
                neighbour_update_flags.remove(BlockFlags::NOTIFY_NEIGHBORS);
                neighbour_update_flags.remove(BlockFlags::SKIP_REDSTONE_WIRE_STATE_REPLACEMENT);
                self.block_registry.prepare(
                    self,
                    position,
                    old_block,
                    replaced_block_state_id,
                    neighbour_update_flags,
                );
                self.block_registry
                    .update_neighbors(self, position, neighbour_update_flags);
                self.block_registry.prepare(
                    self,
                    position,
                    new_block,
                    block_state_id,
                    neighbour_update_flags,
                );
            }

            self.villager_poi
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .update_block(*position, new_block);

            if is_new_block {
                let mut poi = self
                    .portal_poi
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                if crate::world::villager_poi::profession_for_block(old_block).is_some() {
                    poi.remove(position);
                }
                if let Some(poi_type) = crate::world::villager_poi::poi_type_for_block(new_block) {
                    poi.add_with_free_tickets(*position, poi_type, 1);
                }
            }
        }

        let old_state = replaced_block_state_id.to_state();
        let new_state = block_state_id.to_state();
        if pumpkin_world::lighting::LightEngine::has_different_light_properties(
            old_state, new_state,
        ) {
            self.level
                .light_engine
                .update_lighting_at(&self.level, *position);
        }

        replaced_block_state_id
    }

    pub fn break_block(
        self: &Arc<Self>,
        position: &BlockPos,
        cause: Option<&Arc<Player>>,
        flags: BlockFlags,
    ) -> Option<BlockStateId> {
        use crate::plugin::block::block_break::BlockBreakEvent;
        let (broken_block, broken_block_state) = self.get_block_and_state(position);
        if broken_block_state.is_air() {
            return None;
        }

        let mut event = BlockBreakEvent::new(
            cause.cloned(),
            broken_block,
            *position,
            0,
            !flags.contains(BlockFlags::SKIP_DROPS),
        );
        if let Some(server) = self.server.upgrade() {
            server.plugin_manager.fire_blocking(&server, &mut event);
        }
        if event.cancelled {
            return None;
        }

        let mut flags = flags;
        if event.drop {
            flags.remove(BlockFlags::SKIP_DROPS);
        } else {
            flags.insert(BlockFlags::SKIP_DROPS);
        }

        if !flags.contains(BlockFlags::SKIP_DROPS) {
            let tool = cause.as_ref().and_then(|p| {
                let item = p.inventory().held_item();
                if item.is_empty() { None } else { Some(item) }
            });
            let params = crate::world::loot::LootContextParameters {
                tool,
                block_state: Some(broken_block_state),
                position: Some(position.to_f64()),
                killed_by_player: Some(cause.is_some()),
                ..Default::default()
            };
            crate::block::drop_loot(self, broken_block, position, true, &params);
        } else if cause.is_some_and(|p| p.gamemode.load() == pumpkin_util::GameMode::Creative)
            && broken_block.has_tag(&pumpkin_data::tag::Block::MINECRAFT_SHULKER_BOXES)
        {
            if let Some(be) = self.get_block_entity(position)
                && let Some(shulker) = be.as_any().downcast_ref::<crate::block::entities::shulker_box::ShulkerBoxBlockEntity>()
            {
                if !shulker.is_empty() {
                    if let Some(item) = pumpkin_data::item::Item::from_registry_key(broken_block.name) {
                        let mut stack = ItemStack::new(1, item);
                        shulker.apply_to_item_stack(&mut stack);
                        self.drop_stack(position, stack);
                    }
                }
            }
        }

        let new_state_id = if broken_block.is_waterlogged(broken_block_state.id) {
            Block::WATER.default_state.id
        } else {
            Block::AIR.default_state.id
        };

        Some(self.set_block_state(position, new_state_id, flags))
    }

    pub(crate) fn set_block_breaking(
        &self,
        from: &Entity,
        location: BlockPos,
        progress: BlockBreakingProgress,
    ) {
        let chunk_pos = location.chunk_position();
        let (stage, bedrock_event) = match progress {
            BlockBreakingProgress::Start { stage, speed } => (
                stage,
                Some((
                    LevelEvent::BlockStartBreak,
                    bedrock_block_breaking_rate(speed),
                )),
            ),
            BlockBreakingProgress::Update { stage, speed } => (
                stage,
                speed.map(|speed| {
                    (
                        LevelEvent::BlockUpdateBreak,
                        bedrock_block_breaking_rate(speed),
                    )
                }),
            ),
            BlockBreakingProgress::Stop => (-1, Some((LevelEvent::BlockStopBreak, 0))),
        };
        let je_packet = CSetBlockDestroyStage::new(from.entity_id.into(), location, stage as i8);

        if let Some((event_id, data)) = bedrock_event {
            let be_packet = CLevelEvent {
                event_id: VarInt(event_id as i32),
                position: Vector3::new(
                    location.0.x as f32,
                    location.0.y as f32,
                    location.0.z as f32,
                ),
                data: VarInt(data),
            };

            if let Some(player) = self.get_player_by_uuid(from.entity_uuid)
                && let ClientPlatform::Bedrock(client) = player.client.as_ref()
                && let Ok(packet_data) = client.serialize_packet(&be_packet)
            {
                client.try_enqueue_packet(packet_data);
            }

            self.broadcast_to_chunk_except_editioned(
                chunk_pos,
                &[from.entity_uuid],
                &je_packet,
                &be_packet,
            );
        } else {
            self.broadcast_to_chunk_except(chunk_pos, &[from.entity_uuid], &je_packet);
        }
    }

    pub fn update_neighbors_at(
        self: &Arc<Self>,
        block_pos: &BlockPos,
        source_block: &Block,
        except: Option<BlockDirection>,
    ) {
        for direction in BlockDirection::update_order() {
            if except.is_some_and(|d| d == direction) {
                continue;
            }

            let neighbor_pos = block_pos.offset(direction.to_offset());
            let (neighbor_block, neighbor_fluid) = self.get_block_and_fluid(&neighbor_pos);

            let mut event =
                crate::plugin::api::events::block::block_physics::BlockPhysicsEvent::new(
                    neighbor_pos,
                    *block_pos,
                );
            if let Some(server) = self.server.upgrade() {
                server.plugin_manager.fire_blocking(&server, &mut event);
            }
            if event.cancelled {
                continue;
            }

            if let Some(neighbor_pumpkin_block) =
                self.block_registry.get_pumpkin_block(neighbor_block.id)
            {
                neighbor_pumpkin_block.on_neighbor_update(OnNeighborUpdateArgs {
                    world: self,
                    block: neighbor_block,
                    position: &neighbor_pos,
                    source_block,
                    notify: false,
                });
            }

            if let Some(neighbor_pumpkin_fluid) =
                self.block_registry.get_pumpkin_fluid(neighbor_fluid.id)
            {
                neighbor_pumpkin_fluid.on_neighbor_update(
                    self,
                    neighbor_fluid,
                    &neighbor_pos,
                    false,
                );
            }
        }
    }

    /// Updates neighboring blocks of a block
    pub fn update_neighbors(
        self: &Arc<Self>,
        block_pos: &BlockPos,
        except: Option<BlockDirection>,
    ) {
        let source_block = self.get_block(block_pos);
        self.update_neighbors_at(block_pos, source_block, except);
    }

    pub fn update_neighbor(self: &Arc<Self>, neighbor_block_pos: &BlockPos, source_block: &Block) {
        let neighbor_block = self.get_block(neighbor_block_pos);

        let mut event = crate::plugin::api::events::block::block_physics::BlockPhysicsEvent::new(
            *neighbor_block_pos,
            *neighbor_block_pos,
        );
        if let Some(server) = self.server.upgrade() {
            server.plugin_manager.fire_blocking(&server, &mut event);
        }
        if event.cancelled {
            return;
        }

        if let Some(neighbor_pumpkin_block) =
            self.block_registry.get_pumpkin_block(neighbor_block.id)
        {
            neighbor_pumpkin_block.on_neighbor_update(OnNeighborUpdateArgs {
                world: self,
                block: neighbor_block,
                position: neighbor_block_pos,
                source_block,
                notify: false,
            });
        }
    }

    pub fn update_neighbour_for_output_signal(
        self: &Arc<Self>,
        pos: &BlockPos,
        changed_block: &Block,
    ) {
        for direction in BlockDirection::horizontal() {
            let mut relative_pos = pos.offset(direction.to_offset());
            if self.is_loaded(&relative_pos) {
                let state = self.get_block_state(&relative_pos);
                if state.id.to_block() == &Block::COMPARATOR {
                    self.update_neighbor(&relative_pos, changed_block);
                } else if state.is_solid_block() {
                    relative_pos = relative_pos.offset(direction.to_offset());
                    if self.is_loaded(&relative_pos) {
                        let second_state = self.get_block_state(&relative_pos);
                        if second_state.id.to_block() == &Block::COMPARATOR {
                            self.update_neighbor(&relative_pos, changed_block);
                        }
                    }
                }
            }
        }
    }

    pub fn update_from_neighbor_shapes(
        self: &Arc<Self>,
        state_id: BlockStateId,
        pos: &BlockPos,
    ) -> BlockStateId {
        let mut current_state_id = state_id;
        let block = Block::from_state_id(state_id);
        for direction in BlockDirection::all() {
            let neighbor_pos = pos.offset(direction.to_offset());
            let neighbor_state_id = self.get_block_state_id(&neighbor_pos);
            current_state_id = self.block_registry.get_state_for_neighbor_update(
                self,
                block,
                current_state_id,
                pos,
                direction,
                &neighbor_pos,
                neighbor_state_id,
            );
        }
        current_state_id
    }

    pub fn replace_with_state_for_neighbor_update(
        self: &Arc<Self>,
        block_pos: &BlockPos,
        direction: BlockDirection,
        flags: BlockFlags,
    ) {
        let (block, block_state_id) = self.get_block_and_state_id(block_pos);

        if flags.contains(BlockFlags::SKIP_REDSTONE_WIRE_STATE_REPLACEMENT)
            && *block == Block::REDSTONE_WIRE
        {
            return;
        }

        let neighbor_pos = block_pos.offset(direction.to_offset());
        let neighbor_state_id = self.get_block_state_id(&neighbor_pos);

        let new_state_id = self.block_registry.get_state_for_neighbor_update(
            self,
            block,
            block_state_id,
            block_pos,
            direction,
            &neighbor_pos,
            neighbor_state_id,
        );

        if new_state_id != block_state_id {
            if is_air(new_state_id) {
                self.break_block(block_pos, None, flags | BlockFlags::NOTIFY_ALL);
            } else {
                self.set_block_state(block_pos, new_state_id, flags);
            }
        }
    }

    pub fn schedule_block_tick(
        &self,
        block: &Block,
        block_pos: BlockPos,
        delay: u8,
        priority: TickPriority,
    ) {
        self.level
            .schedule_block_tick(block, block_pos, delay, priority);
    }

    pub fn schedule_fluid_tick(
        &self,
        fluid: &Fluid,
        block_pos: BlockPos,
        delay: u8,
        priority: TickPriority,
    ) {
        self.level
            .schedule_fluid_tick(fluid, block_pos, delay, priority);
    }

    pub fn is_block_tick_scheduled(&self, block_pos: &BlockPos, block: &Block) -> bool {
        self.level.is_block_tick_scheduled(block_pos, block)
    }

    pub fn is_fluid_tick_scheduled(&self, block_pos: &BlockPos, fluid: &Fluid) -> bool {
        self.level.is_fluid_tick_scheduled(block_pos, fluid)
    }

    /// Close container screens for all players who have a container open at the given block position.
    pub fn close_container_screens_at(&self, position: &BlockPos) {
        let players = self.players.load();
        for player in players.iter() {
            if player.open_container_pos.load() == Some(*position) {
                player.close_handled_screen();
            }
        }
    }

    pub fn drop_stack(self: &Arc<Self>, pos: &BlockPos, stack: ItemStack) {
        if stack.is_empty() {
            return;
        }

        let half_height = f64::from(EntityType::ITEM.dimension[1]) / 2.0;
        let spawn_pos = {
            let mut r = rand::rng();
            Vector3::new(
                f64::from(pos.0.x) + 0.5 + r.random_range(-0.25..0.25),
                f64::from(pos.0.y) + 0.5 + r.random_range(-0.25..0.25) - half_height,
                f64::from(pos.0.z) + 0.5 + r.random_range(-0.25..0.25),
            )
        };

        let entity = Entity::new(self.clone(), spawn_pos, &EntityType::ITEM);
        let mut item_event = crate::plugin::api::events::entity::item_spawn::ItemSpawnEvent::new(
            entity.entity_id,
            spawn_pos,
            stack.item.registry_key.to_string(),
        );
        if let Some(server) = self.server.upgrade() {
            server
                .plugin_manager
                .fire_blocking(&server, &mut item_event);
        }
        if item_event.cancelled {
            return;
        }

        let item_entity = Arc::new(ItemEntity::new(entity, stack));
        self.spawn_entity(item_entity);
    }

    pub fn drop_stack_from_face(
        self: &Arc<Self>,
        pos: &BlockPos,
        face: BlockDirection,
        stack: ItemStack,
    ) {
        if stack.is_empty() {
            return;
        }

        let offset = face.to_offset();
        let step_x = offset.x;
        let step_y = offset.y;
        let step_z = offset.z;

        let half_width = f64::from(EntityType::ITEM.dimension[0]) / 2.0;
        let half_height = f64::from(EntityType::ITEM.dimension[1]) / 2.0;

        let (spawn_pos, velocity) = {
            let mut r = rand::rng();
            let x = f64::from(pos.0.x)
                + 0.5
                + if step_x == 0 {
                    r.random_range(-0.25..0.25)
                } else {
                    f64::from(step_x) * (0.5 + half_width)
                };
            let y = f64::from(pos.0.y)
                + 0.5
                + if step_y == 0 {
                    r.random_range(-0.25..0.25)
                } else {
                    f64::from(step_y) * (0.5 + half_height)
                }
                - half_height;
            let z = f64::from(pos.0.z)
                + 0.5
                + if step_z == 0 {
                    r.random_range(-0.25..0.25)
                } else {
                    f64::from(step_z) * (0.5 + half_width)
                };

            let delta_x = if step_x == 0 {
                r.random_range(-0.1..0.1)
            } else {
                f64::from(step_x) * 0.1
            };
            let delta_y = if step_y == 0 {
                r.random_range(0.0..0.1)
            } else {
                f64::from(step_y) * 0.1 + 0.1
            };
            let delta_z = if step_z == 0 {
                r.random_range(-0.1..0.1)
            } else {
                f64::from(step_z) * 0.1
            };

            (
                Vector3::new(x, y, z),
                Vector3::new(delta_x, delta_y, delta_z),
            )
        };

        let entity = Entity::new(self.clone(), spawn_pos, &EntityType::ITEM);
        let mut item_event = crate::plugin::api::events::entity::item_spawn::ItemSpawnEvent::new(
            entity.entity_id,
            spawn_pos,
            stack.item.registry_key.to_string(),
        );
        if let Some(server) = self.server.upgrade() {
            server
                .plugin_manager
                .fire_blocking(&server, &mut item_event);
        }
        if item_event.cancelled {
            return;
        }

        let item_entity = Arc::new(ItemEntity::new_with_velocity(entity, stack, velocity, 10));
        self.spawn_entity(item_entity);
    }

    pub fn strike_lightning(self: &Arc<Self>, pos: Vector3<f64>, effect_only: bool) {
        use uuid::Uuid;
        let server_ref = self.server.upgrade();
        if let Some(server_ref) = server_ref {
            let mut event =
                crate::plugin::api::events::world::lightning_strike::LightningStrikeEvent::new(
                    pos,
                    effect_only,
                );
            server_ref
                .plugin_manager
                .fire_blocking(&server_ref, &mut event);
            if event.cancelled {
                return;
            }
        }

        let lightning = crate::entity::r#type::from_type(
            &EntityType::LIGHTNING_BOLT,
            pos,
            self,
            Uuid::new_v4(),
        );

        if let Some(bolt) = lightning
            .cast_any()
            .downcast_ref::<crate::entity::lightning::LightningBoltEntity>()
        {
            bolt.set_visual_only(effect_only);
        }

        self.spawn_entity(lightning);
    }

    /* ItemScatterer.java */
    pub fn scatter_inventory(
        self: &Arc<Self>,
        position: &BlockPos,
        inventory: &Arc<dyn Inventory>,
    ) {
        for i in 0..inventory.size() {
            self.scatter_stack(
                f64::from(position.0.x),
                f64::from(position.0.y),
                f64::from(position.0.z),
                inventory.remove_stack(i),
            );
        }
    }

    pub fn scatter_stack(self: &Arc<Self>, x: f64, y: f64, z: f64, mut stack: ItemStack) {
        const TRIANGULAR_DEVIATION: f64 = 0.114_850_001_711_398_36;

        const XZ_MODE: f64 = 0.0;
        const Y_MODE: f64 = 0.2;

        let width = f64::from(EntityType::ITEM.dimension[0]);
        let half_width = width / 2.0;
        let spawn_area = 1.0 - width;

        let mut rng = Xoroshiro::from_seed(get_seed());

        // TODO: Use world random here: world.random.nextDouble()
        let x = rng.next_f64().mul_add(spawn_area, x.floor()) + half_width;
        let y = rng.next_f64().mul_add(spawn_area, y.floor());
        let z = rng.next_f64().mul_add(spawn_area, z.floor()) + half_width;

        while !stack.is_empty() {
            let item = stack.split((rng.next_bounded_i32(21) + 10) as u8);
            let velocity = Vector3::new(
                rng.next_triangular(XZ_MODE, TRIANGULAR_DEVIATION),
                rng.next_triangular(Y_MODE, TRIANGULAR_DEVIATION),
                rng.next_triangular(XZ_MODE, TRIANGULAR_DEVIATION),
            );

            let entity = Entity::new(self.clone(), Vector3::new(x, y, z), &EntityType::ITEM);
            let entity = Arc::new(ItemEntity::new_with_velocity(entity, item, velocity, 10));
            self.spawn_entity(entity);
        }
    }
    /* End ItemScatterer.java */

    pub fn sync_world_event(&self, world_event: WorldEvent, position: BlockPos, data: i32) {
        let chunk_pos = position.chunk_position();
        self.broadcast_to_chunk(
            chunk_pos,
            &CWorldEvent::new(world_event as i32, position, data, false),
        );
    }

    pub fn set_block_destroy_stage(&self, entity_id: i32, location: BlockPos, stage: i8) {
        let chunk_pos = location.chunk_position();
        let packet = CSetBlockDestroyStage::new(entity_id.into(), location, stage);
        self.broadcast_to_chunk(chunk_pos, &packet);
    }
}

/// Converts a block-breaking `speed` value (progress-per-tick fraction) to the Bedrock
/// `LevelEvent` `data` field expected by the client.
///
/// Vanilla computes this as `Math.ceil(1.0 / speed)`.
pub(crate) fn bedrock_block_breaking_rate(speed: f32) -> i32 {
    (speed.clamp(0.0, 1.0) * f32::from(u16::MAX)) as i32
}
