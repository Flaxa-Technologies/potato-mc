use pumpkin_data::sound::{Sound, SoundCategory};
use pumpkin_protocol::bedrock::client::level_sound_event::CLevelSoundEvent;
use pumpkin_protocol::codec::data_component::data_to_proto_sound;
use pumpkin_protocol::codec::var_int::VarInt;
use pumpkin_protocol::java::client::play::CSoundEffect;
use pumpkin_protocol::IdOr;
use pumpkin_util::math::position::BlockPos;
use pumpkin_util::math::vector3::Vector3;
use rand::RngExt;

use crate::entity::player::Player;
use crate::entity::EntityBase;
use crate::net::ClientPlatform;
use crate::world::chunker::is_within_view_distance;

use super::World;

impl World {
    pub fn play_sound(&self, sound: Sound, category: SoundCategory, position: &Vector3<f64>) {
        self.play_sound_raw(sound as u16, category, position, 1.0, 1.0);
    }

    pub fn play_sound_event(
        &self,
        sound: &pumpkin_data::data_component_impl::IdOr<
            pumpkin_data::data_component_impl::SoundEvent,
        >,
        category: SoundCategory,
        position: &Vector3<f64>,
    ) {
        let seed = rand::rng().random::<f64>();
        let packet = CSoundEffect::new(
            data_to_proto_sound(sound),
            category,
            position,
            1.0,
            1.0,
            seed,
        );
        self.broadcast_packet_all(&packet);
    }

    pub fn play_sound_event_expect(
        &self,
        player: &Player,
        sound: &pumpkin_data::data_component_impl::IdOr<
            pumpkin_data::data_component_impl::SoundEvent,
        >,
        category: SoundCategory,
        position: &Vector3<f64>,
    ) {
        let seed = rand::rng().random::<f64>();
        let packet = CSoundEffect::new(
            data_to_proto_sound(sound),
            category,
            position,
            1.0,
            1.0,
            seed,
        );
        self.broadcast_packet_except(&[player.gameprofile.id], &packet);
    }

    pub fn play_sound_fine(
        &self,
        sound: Sound,
        category: SoundCategory,
        position: &Vector3<f64>,
        volume: f32,
        pitch: f32,
    ) {
        self.play_sound_raw(sound as u16, category, position, volume, pitch);
    }

    /// Plays a custom sound event by identifier for all players in range.
    pub fn play_custom_sound(
        &self,
        sound_name: &str,
        category: SoundCategory,
        position: &Vector3<f64>,
        volume: f32,
        pitch: f32,
    ) {
        let seed = rand::random::<f64>();
        let packet = CSoundEffect::new(
            pumpkin_protocol::IdOr::Value(pumpkin_protocol::SoundEvent {
                sound_name: sound_name.into(),
                range: None,
            }),
            category,
            position,
            volume,
            pitch,
            seed,
        );
        self.broadcast_packet_all(&packet);
    }

    /// Plays a Bedrock level sound for players close enough to hear it.
    pub fn play_bedrock_level_sound(
        &self,
        sound_id: &str,
        position: &Vector3<f64>,
        extra_data: i32,
    ) {
        let packet = CLevelSoundEvent {
            sound_event: sound_id.to_string(),
            position: Vector3::new(position.x as f32, position.y as f32, position.z as f32),
            data: VarInt(extra_data),
            actor_identifier: String::new(),
            is_baby: false,
            is_global: false,
            actor_unique_id: 0,
            fire_at_position: None,
        };
        let chunk_pos = BlockPos::floored_v(*position).chunk_position();

        for player in self.players.load().iter() {
            if is_within_view_distance(chunk_pos, player.get_entity().chunk_pos.load(), 1)
                && let ClientPlatform::Bedrock(client) = player.client.as_ref()
                && let Ok(data) = client.serialize_packet(&packet)
            {
                client.try_enqueue_packet(data);
            }
        }
    }

    pub fn play_sound_expect(
        &self,
        player: &Player,
        sound: Sound,
        category: SoundCategory,
        position: &Vector3<f64>,
    ) {
        self.play_sound_raw_expect(player, sound as u16, category, position, 1.0, 1.0);
    }

    pub fn play_sound_raw(
        &self,
        sound_id: u16,
        category: SoundCategory,
        position: &Vector3<f64>,
        volume: f32,
        pitch: f32,
    ) {
        let seed = rand::rng().random::<f64>();
        let packet = CSoundEffect::new(IdOr::Id(sound_id), category, position, volume, pitch, seed);

        // Calculate the number of chunks the sound can be heard from based on its volume.
        let audible_chunks = f64::from(volume.max(1.0)).ceil() as i32;
        let chunk_pos = BlockPos::floored_v(*position).chunk_position();

        let players = self.players.load();
        let recipients = players.iter().filter(|p| {
            let center = p.get_entity().chunk_pos.load();
            // If the sound reaches their chunk, send it!
            is_within_view_distance(chunk_pos, center, audible_chunks)
        });

        let recipients_by_version = Self::collect_java_recipients_by_version(recipients);
        Self::broadcast_java_grouped(&packet, recipients_by_version);
    }

    pub fn play_sound_raw_expect(
        &self,
        player: &Player,
        sound_id: u16,
        category: SoundCategory,
        position: &Vector3<f64>,
        volume: f32,
        pitch: f32,
    ) {
        let seed = rand::rng().random::<f64>();
        let packet = CSoundEffect::new(IdOr::Id(sound_id), category, position, volume, pitch, seed);

        let audible_chunks = f64::from(volume.max(1.0)).ceil() as i32;
        let chunk_pos = BlockPos::floored_v(*position).chunk_position();

        let players = self.players.load();
        let recipients = players.iter().filter(|p| {
            // Skip the expected player
            if p.gameprofile.id == player.gameprofile.id {
                return false;
            }

            let center = p.get_entity().chunk_pos.load();
            is_within_view_distance(chunk_pos, center, audible_chunks)
        });

        let recipients_by_version = Self::collect_java_recipients_by_version(recipients);
        Self::broadcast_java_grouped(&packet, recipients_by_version);
    }

    pub fn play_block_sound(&self, sound: Sound, category: SoundCategory, position: BlockPos) {
        let new_vec = Vector3::new(
            f64::from(position.0.x) + 0.5,
            f64::from(position.0.y) + 0.5,
            f64::from(position.0.z) + 0.5,
        );
        self.play_sound(sound, category, &new_vec);
    }

    pub fn play_block_sound_expect(
        &self,
        player: &Player,
        sound: Sound,
        category: SoundCategory,
        position: BlockPos,
    ) {
        let new_vec = Vector3::new(
            f64::from(position.0.x) + 0.5,
            f64::from(position.0.y) + 0.5,
            f64::from(position.0.z) + 0.5,
        );
        self.play_sound_expect(player, sound, category, &new_vec);
    }
}
