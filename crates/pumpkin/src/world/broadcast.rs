use bytes::BufMut;
use pumpkin_data::world::RAW;
use pumpkin_protocol::bedrock::server::text::SText;
use pumpkin_protocol::codec::var_int::VarInt;
use pumpkin_protocol::java::client::play::{
    CDisguisedChatMessage, CSetEntityMetadata, CSystemChatMessage, Metadata,
};
use pumpkin_protocol::java::server::play::SChatMessage;
use pumpkin_protocol::{BClientPacket, ClientPacket};
use pumpkin_util::math::vector2::Vector2;
use pumpkin_util::text::{TextComponent, TextComponentBase};
use pumpkin_util::version::JavaMinecraftVersion;
use std::collections::BTreeMap;
use std::sync::Arc;
use tracing::error;
use uuid::Uuid;

use crate::entity::player::Player;
use crate::entity::{Entity, EntityBase};
use crate::net::bedrock::BedrockClient;
use crate::net::java::JavaClient;
use crate::net::ClientPlatform;
use crate::world::chunker::{get_view_distance, is_within_view_distance};

use super::World;

impl World {
    pub fn send_to_tracking_players<P: ClientPacket + Sync>(&self, entity: &Entity, packet: &P) {
        if let Some(tracked) = self.entity_tracker.get_tracked_entity(entity.entity_id) {
            tracked.send_to_tracking_players(packet, self);
        }
    }

    pub fn send_to_tracking_players_bedrock<P: BClientPacket + Sync>(
        &self,
        entity: &Entity,
        packet: &P,
    ) {
        if let Some(tracked) = self.entity_tracker.get_tracked_entity(entity.entity_id) {
            tracked.send_to_tracking_players_bedrock(packet, self);
        }
    }

    pub fn send_to_tracking_players_editioned<J: ClientPacket + Sync, B: BClientPacket + Sync>(
        &self,
        entity: &Entity,
        je_packet: &J,
        be_packet: &B,
    ) {
        if let Some(tracked) = self.entity_tracker.get_tracked_entity(entity.entity_id) {
            tracked.send_to_tracking_players_editioned(je_packet, be_packet, self);
        }
    }

    pub fn send_to_tracking_players_and_self<P: ClientPacket + Sync>(
        &self,
        entity: &Entity,
        packet: &P,
    ) {
        if let Some(tracked) = self.entity_tracker.get_tracked_entity(entity.entity_id) {
            tracked.send_to_tracking_players_and_self(packet, self);
        }
    }

    pub fn send_to_tracking_players_and_self_editioned<
        J: ClientPacket + Sync,
        B: BClientPacket + Sync,
    >(
        &self,
        entity: &Entity,
        je_packet: &J,
        be_packet: &B,
    ) {
        if let Some(tracked) = self.entity_tracker.get_tracked_entity(entity.entity_id) {
            tracked.send_to_tracking_players_and_self_editioned(je_packet, be_packet, self);
        }
    }

    pub fn send_to_tracking_players_filtered<P: ClientPacket + Sync, F: Fn(&Player) -> bool>(
        &self,
        entity: &Entity,
        packet: &P,
        filter: F,
    ) {
        if let Some(tracked) = self.entity_tracker.get_tracked_entity(entity.entity_id) {
            tracked.send_to_tracking_players_filtered(packet, self, filter);
        }
    }

    pub fn send_to_tracking_players_filtered_editioned<
        J: ClientPacket + Sync,
        B: BClientPacket + Sync,
        F: Fn(&Player) -> bool,
    >(
        &self,
        entity: &Entity,
        je_packet: &J,
        be_packet: &B,
        filter: F,
    ) {
        if let Some(tracked) = self.entity_tracker.get_tracked_entity(entity.entity_id) {
            tracked.send_to_tracking_players_filtered_editioned(je_packet, be_packet, self, filter);
        }
    }

    #[must_use]
    pub fn is_tracked_by_any_player(&self, entity: &Entity) -> bool {
        self.entity_tracker
            .is_tracked_by_any_player(entity.entity_id)
    }

    pub(crate) fn collect_java_recipients_by_version<'a>(
        players: impl Iterator<Item = &'a Arc<Player>>,
    ) -> BTreeMap<JavaMinecraftVersion, Vec<&'a JavaClient>> {
        let mut recipients_by_version: BTreeMap<JavaMinecraftVersion, Vec<&'a JavaClient>> =
            BTreeMap::new();
        for player in players {
            if let ClientPlatform::Java(java_client) = player.client.as_ref() {
                recipients_by_version
                    .entry(java_client.version.load())
                    .or_default()
                    .push(java_client);
            }
        }
        recipients_by_version
    }

    pub fn broadcast_java_clients<'a, P: ClientPacket>(
        packet: &P,
        recipients: impl Iterator<Item = &'a JavaClient>,
    ) {
        let mut recipients_by_version: BTreeMap<JavaMinecraftVersion, Vec<&JavaClient>> =
            BTreeMap::new();
        for client in recipients {
            recipients_by_version
                .entry(client.version.load())
                .or_default()
                .push(client);
        }
        Self::broadcast_java_grouped(packet, recipients_by_version);
    }

    pub(crate) fn broadcast_java_grouped<P: ClientPacket>(
        packet: &P,
        recipients_by_version: BTreeMap<JavaMinecraftVersion, Vec<&JavaClient>>,
    ) {
        for (version, recipients) in recipients_by_version {
            let packet_data = match JavaClient::serialize_packet_for_version(packet, version) {
                Ok(packet_data) => packet_data,
                Err(pumpkin_protocol::ser::WritingError::UnsupportedVersion(_)) => {
                    continue;
                }
                Err(err) => {
                    error!(
                        "Failed to serialize packet {} for version {:?}: {}",
                        std::any::type_name::<P>(),
                        version,
                        err
                    );
                    continue;
                }
            };

            for recipient in recipients {
                recipient.try_enqueue_packet(packet_data.clone());
            }
        }
    }

    pub(crate) fn broadcast_bedrock_grouped<'a, P: BClientPacket>(
        packet: &P,
        recipients: impl Iterator<Item = &'a Arc<BedrockClient>>,
    ) {
        for recipient in recipients {
            match recipient.serialize_packet(packet) {
                Ok(packet_data) => recipient.try_enqueue_packet(packet_data),
                Err(err) => {
                    error!(
                        "Failed to serialize bedrock packet {}: {}",
                        std::any::type_name::<P>(),
                        err
                    );
                }
            }
        }
    }

    /// Broadcasts a packet to all connected players within the world.
    /// Please avoid this as we want to replace it with `broadcast_editioned`
    pub fn broadcast_packet_all<P: ClientPacket>(&self, packet: &P) {
        let players = self.players.load();
        let recipients_by_version = Self::collect_java_recipients_by_version(players.iter());
        Self::broadcast_java_grouped(packet, recipients_by_version);
    }

    pub fn broadcast_system_message(&self, message: &TextComponent, overlay: bool) {
        let je_packet = CSystemChatMessage::new(message, overlay);
        let be_packet = Self::component_to_bedrock_text(message);
        self.broadcast_editioned(&je_packet, &be_packet);
    }

    fn component_to_bedrock_text(message: &TextComponent) -> SText<'static> {
        match &*message.0.content {
            pumpkin_util::text::TextContent::Translate {
                translate,
                bedrock_translate,
                with,
            } => {
                let key = bedrock_translate.as_deref().unwrap_or(translate.as_ref());
                let parameters = with
                    .iter()
                    .map(pumpkin_util::text::TextComponentBase::to_bedrock_string)
                    .collect();
                SText::translation(key.to_string(), parameters)
            }
            _ => SText::system_message(
                message
                    .0
                    .to_bedrock_legacy(pumpkin_util::translation::Locale::EnUs),
            ),
        }
    }

    pub fn broadcast_message(
        &self,
        message: &TextComponent,
        sender_name: &TextComponent,
        chat_type: u8,
        target_name: Option<&TextComponent>,
    ) {
        let be_packet = SText::new(message.clone().get_text(), sender_name.clone().get_text());
        let je_packet =
            CDisguisedChatMessage::new(message, (chat_type + 1).into(), sender_name, target_name);

        self.broadcast_editioned(&je_packet, &be_packet);
    }

    // This should replace broadcast_packet_all at some point
    pub fn broadcast_editioned<J: ClientPacket, B: BClientPacket>(
        &self,
        je_packet: &J,
        be_packet: &B,
    ) {
        let players = self.players.load();
        let je_recipients_by_version = Self::collect_java_recipients_by_version(players.iter());

        Self::broadcast_java_grouped(je_packet, je_recipients_by_version);
        Self::broadcast_bedrock_grouped(
            be_packet,
            players.iter().filter_map(|p| match p.client.as_ref() {
                ClientPlatform::Bedrock(be) => Some(be),
                ClientPlatform::Java(_) => None,
            }),
        );
    }

    pub fn broadcast_bedrock_all<B: BClientPacket>(&self, packet: &B) {
        let players = self.players.load();
        Self::broadcast_bedrock_grouped(
            packet,
            players.iter().filter_map(|p| match p.client.as_ref() {
                ClientPlatform::Bedrock(be) => Some(be),
                ClientPlatform::Java(_) => None,
            }),
        );
    }

    pub fn broadcast_chat_message(
        &self,
        message: &crate::net::chat::PlayerChatMessage,
        is_filtered: impl Fn(&Player) -> bool,
        sender_player: Option<&Arc<Player>>,
        chat_type: VarInt,
        sender_name: &TextComponent,
        target_name: Option<&TextComponent>,
    ) {
        let tracked = crate::net::chat::OutgoingChatMessage::create(message.clone());
        let mut was_fully_filtered = false;

        let players = self.players.load();
        for player in players.iter() {
            let filtered = is_filtered(player);
            tracked.send_to_player(player, filtered, chat_type, sender_name, target_name);
            was_fully_filtered |= filtered && message.is_fully_filtered();
        }

        if was_fully_filtered && let Some(sender) = sender_player {
            let filter_notice =
                TextComponent::translate(pumpkin_data::translation::java::CHAT_FILTERED_FULL, [])
                    .color_named(pumpkin_util::text::color::NamedColor::Red)
                    .italic();
            sender.send_system_message(&filter_notice);
        }
    }

    pub fn broadcast_secure_player_chat(
        &self,
        sender: &Arc<Player>,
        chat_message: &SChatMessage<'_>,
        decorated_message: &TextComponent,
    ) {
        let messages_sent: i32 = sender
            .chat_session
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .messages_sent;
        let sender_last_seen = {
            let cache = sender
                .signature_cache
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            cache.last_seen.as_ref().to_vec()
        };

        let link = crate::net::chat::SignedMessageLink::new(
            messages_sent,
            sender.gameprofile.id,
            Uuid::nil(),
        );
        let signed_body = crate::net::chat::SignedMessageBody::new(
            chat_message.message.to_string(),
            chat_message.timestamp,
            chat_message.salt,
            sender_last_seen,
        );
        let player_chat_msg = crate::net::chat::PlayerChatMessage::new(
            link,
            chat_message.signature.map(std::convert::Into::into),
            signed_body,
            Some(decorated_message.clone()),
            crate::net::chat::FilterMask::PassThrough,
        );

        self.broadcast_chat_message(
            &player_chat_msg,
            Player::is_text_filtering_enabled,
            Some(sender),
            (RAW + 1).into(),
            &TextComponent::empty(),
            None,
        );

        sender
            .chat_session
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .messages_sent += 1;
    }

    pub fn broadcast_packet_except_editioned<J: ClientPacket, B: BClientPacket>(
        &self,
        except: &[uuid::Uuid],
        je_packet: &J,
        be_packet: &B,
    ) {
        let players = self.players.load();
        let mut java_recipients = Vec::new();
        let mut bedrock_recipients = Vec::new();

        for p in players.iter() {
            if except.contains(&p.gameprofile.id) {
                continue;
            }
            match p.client.as_ref() {
                ClientPlatform::Java(_) => java_recipients.push(p),
                ClientPlatform::Bedrock(be_client) => bedrock_recipients.push(be_client),
            }
        }

        let recipients_by_version =
            Self::collect_java_recipients_by_version(java_recipients.into_iter());
        Self::broadcast_java_grouped(je_packet, recipients_by_version);
        Self::broadcast_bedrock_grouped(be_packet, bedrock_recipients.into_iter());
    }

    /// Broadcasts the skin layers of a player, encoding the metadata for each Java client's own
    /// protocol version since the tracked data index differs between versions.
    pub fn broadcast_skin_parts<B: BClientPacket>(
        &self,
        except: &[uuid::Uuid],
        entity_id: i32,
        skin_parts: u8,
        be_packet: &B,
    ) {
        let players = self.players.load();
        let mut java_recipients = Vec::new();
        let mut bedrock_recipients = Vec::new();

        for p in players.iter() {
            if except.contains(&p.gameprofile.id) {
                continue;
            }
            match p.client.as_ref() {
                ClientPlatform::Java(_) => java_recipients.push(p),
                ClientPlatform::Bedrock(be_client) => bedrock_recipients.push(be_client),
            }
        }

        let recipients_by_version =
            Self::collect_java_recipients_by_version(java_recipients.into_iter());

        for (version, recipients) in recipients_by_version {
            if version < JavaMinecraftVersion::V_1_21 {
                continue;
            }
            let mut buf = Vec::new();
            for meta in [
                Metadata::new(
                    pumpkin_data::tracked_data::player::PLAYER_MODE_CUSTOMISATION,
                    skin_parts,
                ),
                Metadata::new(
                    pumpkin_data::tracked_data::player::PLAYER_MODE_CUSTOMIZATION_ID,
                    skin_parts,
                ),
            ] {
                let _ = meta.write(&mut buf, &version);
            }
            buf.put_u8(255);
            let packet = CSetEntityMetadata::new(entity_id.into(), buf.into());
            if let Ok(packet_data) = JavaClient::serialize_packet_for_version(&packet, version) {
                for recipient in recipients {
                    recipient.try_enqueue_packet(packet_data.clone());
                }
            }
        }

        Self::broadcast_bedrock_grouped(be_packet, bedrock_recipients.into_iter());
    }

    /// Broadcasts a packet to all connected players within the world, excluding the specified players.
    pub fn broadcast_packet_except<P: ClientPacket>(&self, except: &[uuid::Uuid], packet: &P) {
        let players = self.players.load();
        let recipients_by_version = Self::collect_java_recipients_by_version(
            players
                .iter()
                .filter(|candidate| !except.contains(&candidate.gameprofile.id)),
        );
        Self::broadcast_java_grouped(packet, recipients_by_version);
    }

    /// Broadcasts a packet to all players who currently have the target chunk loaded.
    /// This uses highly optimized Chebyshev distance math (Chunk Grid) instead of floating point distance checks.
    pub fn broadcast_to_chunk<P: ClientPacket>(&self, chunk_pos: Vector2<i32>, packet: &P) {
        let players = self.players.load();

        let recipients = players.iter().filter(|p| {
            let center = p.get_entity().chunk_pos.load();
            let view_distance = get_view_distance(p).get() as i32;

            // Chebyshev distance (Minecraft's chunk loading shape)
            is_within_view_distance(chunk_pos, center, view_distance)
        });

        let recipients_by_version = Self::collect_java_recipients_by_version(recipients);
        Self::broadcast_java_grouped(packet, recipients_by_version);
    }

    pub fn broadcast_to_chunk_bedrock<P: BClientPacket>(
        &self,
        chunk_pos: Vector2<i32>,
        packet: &P,
    ) {
        let players = self.players.load();
        let recipients = players.iter().filter_map(|player| {
            let center = player.get_entity().chunk_pos.load();
            let view_distance = get_view_distance(player).get() as i32;
            if is_within_view_distance(chunk_pos, center, view_distance)
                && let ClientPlatform::Bedrock(client) = player.client.as_ref()
            {
                return Some(client);
            }
            None
        });
        Self::broadcast_bedrock_grouped(packet, recipients);
    }

    pub fn broadcast_to_chunk_editioned<J: ClientPacket, B: BClientPacket>(
        &self,
        chunk_pos: Vector2<i32>,
        je_packet: &J,
        be_packet: &B,
    ) {
        let players = self.players.load();
        let mut java_recipients = Vec::new();
        let mut bedrock_recipients = Vec::new();

        let recipients = players.iter().filter(|p| {
            let center = p.get_entity().chunk_pos.load();
            let view_distance = get_view_distance(p).get() as i32;
            is_within_view_distance(chunk_pos, center, view_distance)
        });

        for p in recipients {
            match p.client.as_ref() {
                ClientPlatform::Java(_) => java_recipients.push(p),
                ClientPlatform::Bedrock(be_client) => bedrock_recipients.push(be_client),
            }
        }

        let recipients_by_version =
            Self::collect_java_recipients_by_version(java_recipients.into_iter());
        Self::broadcast_java_grouped(je_packet, recipients_by_version);
        Self::broadcast_bedrock_grouped(be_packet, bedrock_recipients.into_iter());
    }

    /// Broadcasts a packet to chunk watchers, excluding specific players.
    pub fn broadcast_to_chunk_except<P: ClientPacket>(
        &self,
        chunk_pos: Vector2<i32>,
        except: &[uuid::Uuid],
        packet: &P,
    ) {
        let players = self.players.load();

        let recipients = players.iter().filter(|p| {
            if except.contains(&p.get_entity().entity_uuid) {
                return false;
            }
            let center = p.get_entity().chunk_pos.load();
            let view_distance = get_view_distance(p).get() as i32;

            is_within_view_distance(chunk_pos, center, view_distance)
        });

        let recipients_by_version = Self::collect_java_recipients_by_version(recipients);
        Self::broadcast_java_grouped(packet, recipients_by_version);
    }

    pub fn broadcast_to_chunk_except_editioned<J: ClientPacket, B: BClientPacket>(
        &self,
        chunk_pos: Vector2<i32>,
        except: &[uuid::Uuid],
        je_packet: &J,
        be_packet: &B,
    ) {
        let players = self.players.load();
        let recipients = players.iter().filter(|p| {
            if except.contains(&p.get_entity().entity_uuid) {
                return false;
            }
            let center = p.get_entity().chunk_pos.load();
            let view_distance = get_view_distance(p).get() as i32;

            is_within_view_distance(chunk_pos, center, view_distance)
        });

        let mut java_recipients = Vec::new();
        let mut bedrock_recipients = Vec::new();

        for p in recipients {
            match p.client.as_ref() {
                ClientPlatform::Java(_) => java_recipients.push(p),
                ClientPlatform::Bedrock(be_client) => bedrock_recipients.push(be_client),
            }
        }

        let je_recipients_by_version =
            Self::collect_java_recipients_by_version(java_recipients.into_iter());
        Self::broadcast_java_grouped(je_packet, je_recipients_by_version);
        Self::broadcast_bedrock_grouped(be_packet, bedrock_recipients.into_iter());
    }
}
