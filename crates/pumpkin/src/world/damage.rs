use pumpkin_data::effect::StatusEffect;
use pumpkin_data::entity::EntityStatus;
use pumpkin_protocol::bedrock::server::actor_event::{ActorEventID, SActorEvent};
use pumpkin_protocol::codec::var_int::VarInt;
use pumpkin_protocol::codec::var_ulong::VarULong;
use pumpkin_protocol::java::client::play::{
    CDamageEvent, CEntityStatus, CRemoveMobEffect, CUpdateMobEffect,
};
use pumpkin_util::math::vector3::Vector3;

use crate::entity::Entity;

use super::World;

impl World {
    /// Broadcasts an entity status update / event to all players tracking the specified entity,
    /// and to the entity itself if it is a player.
    /// Matching Vanilla's `ServerLevel.broadcastEntityEvent(entity, event)`.
    pub fn broadcast_entity_event(
        &self,
        entity: &Entity,
        java_status: EntityStatus,
        bedrock_status: Option<ActorEventID>,
    ) {
        let je_packet = CEntityStatus::new(entity.entity_id, java_status as i8);
        if let Some(be_event) = bedrock_status {
            let be_packet = SActorEvent {
                target_runtime_id: VarULong(entity.entity_id as u64),
                event_id: be_event,
                data: VarInt(0),
                fire_at_position: None,
            };
            self.send_to_tracking_players_and_self_editioned(entity, &je_packet, &be_packet);
        } else {
            self.send_to_tracking_players_and_self(entity, &je_packet);
        }
    }

    /// Broadcasts a damage event to all players tracking the specified entity,
    /// and to the entity itself if it is a player.
    /// Matching Vanilla's `ServerLevel.broadcastDamageEvent(entity, source)`.
    pub fn broadcast_damage_event(
        &self,
        entity: &Entity,
        damage_type_id: i32,
        cause_entity_id: Option<i32>,
        direct_entity_id: Option<i32>,
        position: Option<Vector3<f64>>,
    ) {
        let je_packet = CDamageEvent::new(
            entity.entity_id.into(),
            damage_type_id.into(),
            cause_entity_id.map(Into::into),
            direct_entity_id.map(Into::into),
            position,
        );
        let be_packet = SActorEvent {
            target_runtime_id: VarULong(entity.entity_id as u64),
            event_id: ActorEventID::Hurt,
            data: VarInt(0),
            fire_at_position: None,
        };
        self.send_to_tracking_players_and_self_editioned(entity, &je_packet, &be_packet);
    }

    /// Sends an entity status update to all players tracking the specified entity.
    pub fn send_entity_status(
        &self,
        entity: &Entity,
        java_status: EntityStatus,
        bedrock_status: Option<ActorEventID>,
    ) {
        self.broadcast_entity_event(entity, java_status, bedrock_status);
    }

    pub fn send_remove_mob_effect(&self, entity: &Entity, effect_type: &'static StatusEffect) {
        let je_packet =
            CRemoveMobEffect::new(entity.entity_id.into(), VarInt(i32::from(effect_type.id)));

        let be_packet = pumpkin_protocol::bedrock::client::CMobEffect {
            target_runtime_id: VarULong(entity.entity_id as u64),
            event_id: pumpkin_protocol::bedrock::client::CMobEffect::EVENT_REMOVE,
            effect_id: VarInt(effect_type.to_bedrock_id()),
            effect_amplifier: VarInt(0),
            show_particles: false,
            effect_duration_ticks: VarInt(0),
            tick: VarULong(0),
            ambient: false,
        };
        self.send_to_tracking_players_and_self_editioned(entity, &je_packet, &be_packet);
    }

    pub fn send_add_mob_effect(&self, entity: &Entity, effect: &pumpkin_data::potion::Effect) {
        let mut flags: i8 = 0;
        if effect.ambient {
            flags |= 0x01;
        }
        if effect.show_particles {
            flags |= 0x02;
        }
        if effect.show_icon {
            flags |= 0x04;
        }

        let je_packet = CUpdateMobEffect::new(
            VarInt(entity.entity_id),
            VarInt(i32::from(effect.effect_type.id)),
            VarInt(i32::from(effect.amplifier)),
            VarInt(effect.duration),
            flags,
        );

        let be_packet = pumpkin_protocol::bedrock::client::CMobEffect {
            target_runtime_id: VarULong(entity.entity_id as u64),
            event_id: pumpkin_protocol::bedrock::client::CMobEffect::EVENT_ADD,
            effect_id: VarInt(effect.effect_type.to_bedrock_id()),
            effect_amplifier: VarInt(i32::from(effect.amplifier)),
            show_particles: effect.show_particles,
            effect_duration_ticks: VarInt(effect.duration),
            tick: VarULong(0),
            ambient: effect.ambient,
        };

        self.send_to_tracking_players_and_self_editioned(entity, &je_packet, &be_packet);
    }
}
