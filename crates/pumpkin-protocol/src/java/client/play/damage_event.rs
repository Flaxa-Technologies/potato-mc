use crate::ClientPacket;
use crate::VarInt;
use crate::ser::NetworkWriteExt;
use pumpkin_data::packet::clientbound::play::DAMAGE_EVENT;
use pumpkin_macros::java_packet;
use pumpkin_util::math::vector3::Vector3;
use pumpkin_util::version::JavaMinecraftVersion;

/// Notifies the client that an entity has taken damage.
///
/// This packet is used to trigger damage animations (like the red tint on mobs),
/// directional knockback visuals, and sound effects. It provides the client
/// with specific details about the damage source to ensure the visual feedback
/// matches the cause.
#[java_packet(DAMAGE_EVENT)]
pub struct CDamageEvent {
    /// The Entity ID of the entity taking damage.
    pub entity_id: VarInt,
    /// The ID of the damage type (references the `minecraft:damage_type` registry).
    /// Examples: `magic`, `fall`, `on_fire`, or `arrow`.
    pub source_type_id: VarInt,
    /// The Entity ID of the actual cause of the damage (e.g., the player who shot the arrow).
    /// Set to 0 if there is no specific entity cause.
    pub source_cause_id: VarInt,
    /// The Entity ID of the direct damager (e.g., the arrow entity itself).
    /// Set to 0 if this is the same as the cause or if not applicable.
    pub source_direct_id: VarInt,
    /// The coordinates of the damage source. Used by the client to calculate
    /// the direction of the "damage tilt" camera effect.
    pub source_position: Option<Vector3<f64>>,
}

impl CDamageEvent {
    #[must_use]
    pub fn new(
        entity_id: VarInt,
        source_type_id: VarInt,
        source_cause_id: Option<VarInt>,
        source_direct_id: Option<VarInt>,
        source_position: Option<Vector3<f64>>,
    ) -> Self {
        Self {
            entity_id,
            source_type_id,
            source_cause_id: source_cause_id.map_or(VarInt(0), |id| VarInt(id.0 + 1)),
            source_direct_id: source_direct_id.map_or(VarInt(0), |id| VarInt(id.0 + 1)),
            source_position,
        }
    }
}

impl ClientPacket for CDamageEvent {
    fn write_packet_data(
        &self,
        mut write: impl std::io::Write,
        version: &JavaMinecraftVersion,
    ) -> Result<(), crate::ser::WritingError> {
        write.write_var_int(&self.entity_id)?;
        let mapped_source_type_id = if *version <= JavaMinecraftVersion::V_26_1 {
            // In 26.1 and earlier, the dynamic damage_type registry only had 50 entries (0..=49).
            // `sulfur_cube_hot` (added at 42 in 26.2) does not exist in 26.1, so all entries after 42
            // were shifted up by 1 in 26.2.
            if self.source_type_id.0 == 42 {
                VarInt(31) // on_fire fallback
            } else if self.source_type_id.0 > 42 {
                let shifted = self.source_type_id.0 - 1;
                VarInt(shifted.min(49))
            } else {
                VarInt(self.source_type_id.0.min(49))
            }
        } else {
            self.source_type_id
        };
        write.write_var_int(&mapped_source_type_id)?;
        write.write_var_int(&self.source_cause_id)?;
        write.write_var_int(&self.source_direct_id)?;
        if let Some(pos) = &self.source_position {
            write.write_bool(true)?;
            write.write_f64(pos.x)?;
            write.write_f64(pos.y)?;
            write.write_f64(pos.z)?;
        } else {
            write.write_bool(false)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::java::packet_encoder::serialize_packet;

    #[test]
    fn test_damage_event_serialization() {
        let packet = CDamageEvent::new(
            VarInt(5),
            VarInt(34),
            Some(VarInt(10)),
            Some(VarInt(10)),
            None,
        );
        let bytes = serialize_packet(&packet, &JavaMinecraftVersion::V_26_2).expect("serialize");
        // Packet ID 25 (0x19) in 26.2
        assert_eq!(bytes[0], 0x19);
        // Entity ID 5
        assert_eq!(bytes[1], 5);
        // Source type ID 34
        assert_eq!(bytes[2], 34);
        // Source cause ID 11 (10 + 1)
        assert_eq!(bytes[3], 11);
        // Source direct ID 11 (10 + 1)
        assert_eq!(bytes[4], 11);
        // Has position: false
        assert_eq!(bytes[5], 0);
    }

    #[test]
    fn test_damage_event_remap_v26_1() {
        // Test that wither_skull (id 50 in 26.2) remaps to 49 in 26.1
        let packet = CDamageEvent::new(
            VarInt(5),
            VarInt(50),
            None,
            None,
            None,
        );
        let bytes = serialize_packet(&packet, &JavaMinecraftVersion::V_26_1).expect("serialize");
        assert_eq!(bytes[1], 5); // entity_id
        assert_eq!(bytes[2], 49); // remapped to 49 for 26.1!
    }

    #[test]
    fn test_damage_event_without_causes_with_pos() {
        let packet = CDamageEvent::new(
            VarInt(7),
            VarInt(9),
            None,
            None,
            Some(Vector3::new(10.0, 20.0, 30.0)),
        );
        let bytes = serialize_packet(&packet, &JavaMinecraftVersion::V_26_2).expect("serialize");
        assert_eq!(bytes[0], 0x19);
        assert_eq!(bytes[1], 7);
        assert_eq!(bytes[2], 9);
        // Cause ID 0
        assert_eq!(bytes[3], 0);
        // Direct ID 0
        assert_eq!(bytes[4], 0);
        // Has position: true
        assert_eq!(bytes[5], 1);
        assert_eq!(bytes.len(), 1 + 1 + 1 + 1 + 1 + 1 + 24);
    }
}
