use pumpkin_data::packet::clientbound::play::MOVE_ENTITY_POS_ROT;
use pumpkin_macros::java_packet;
use pumpkin_util::{math::vector3::Vector3, version::JavaMinecraftVersion};

use crate::{
    ClientPacket, ServerPacket, VarInt,
    ser::{NetworkReadExt, NetworkWriteExt, ReadingError, WritingError},
};

#[java_packet(MOVE_ENTITY_POS_ROT)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CUpdateEntityPosRot {
    pub entity_id: VarInt,
    pub delta: Vector3<i16>,
    pub yaw: u8,
    pub pitch: u8,
    pub on_ground: bool,
}

impl CUpdateEntityPosRot {
    #[must_use]
    pub const fn new(
        entity_id: VarInt,
        delta: Vector3<i16>,
        yaw: u8,
        pitch: u8,
        on_ground: bool,
    ) -> Self {
        Self {
            entity_id,
            delta,
            yaw,
            pitch,
            on_ground,
        }
    }
}

impl ClientPacket for CUpdateEntityPosRot {
    fn write_packet_data(
        &self,
        mut write: impl std::io::Write,
        version: &JavaMinecraftVersion,
    ) -> Result<(), WritingError> {
        if *version <= JavaMinecraftVersion::V_1_7_6 {
            write.write_i32_be(self.entity_id.0)?;
        } else {
            write.write_var_int(&self.entity_id)?;
        }
        if *version >= JavaMinecraftVersion::V_26_3 {
            // 26.3+: `properties` VarInt packs onGround (bit 0) and stepCount (bits 1+)
            // For a simple Linear move, stepCount is always 0.
            let properties: i32 = if self.on_ground { 1 } else { 0 }; // stepCount=0, so bits 1+ are 0
            write.write_var_int(&VarInt(properties))?;
            // Linear VecDelta: 3 × i16
            write.write_i16_be(self.delta.x)?;
            write.write_i16_be(self.delta.y)?;
            write.write_i16_be(self.delta.z)?;
        } else if *version >= JavaMinecraftVersion::V_1_9 {
            write.write_i16_be(self.delta.x)?;
            write.write_i16_be(self.delta.y)?;
            write.write_i16_be(self.delta.z)?;
        } else {
            write.write_i8((self.delta.x / 128) as i8)?;
            write.write_i8((self.delta.y / 128) as i8)?;
            write.write_i8((self.delta.z / 128) as i8)?;
        }
        write.write_u8(self.yaw)?;
        write.write_u8(self.pitch)?;
        if *version >= JavaMinecraftVersion::V_1_8 && *version < JavaMinecraftVersion::V_26_3 {
            // In 26.3+ onGround is packed in the properties VarInt above
            write.write_bool(self.on_ground)?;
        }
        Ok(())
    }
}

impl<'a> ServerPacket<'a> for CUpdateEntityPosRot {
    fn read(bytebuf: &mut &'a [u8], version: &JavaMinecraftVersion) -> Result<Self, ReadingError> {
        let entity_id = if *version <= JavaMinecraftVersion::V_1_7_6 {
            VarInt(bytebuf.get_i32_be()?)
        } else {
            bytebuf.get_var_int()?
        };
        let (delta, on_ground) = if *version >= JavaMinecraftVersion::V_26_3 {
            // 26.3+: properties VarInt packs onGround (bit 0) + stepCount (bits 1+)
            let properties = bytebuf.get_var_int()?.0;
            let on_ground = (properties & 1) != 0;
            let step_count = (properties as u32 >> 1) as i32;
            if step_count > 0 {
                // Stepped VecDelta: consume step_count × (VarInt ticks + 3×i16) and use last delta
                let mut dx: i16 = 0;
                let mut dy: i16 = 0;
                let mut dz: i16 = 0;
                for _ in 0..step_count {
                    let _ticks = bytebuf.get_var_int()?;
                    dx = bytebuf.get_i16_be()?;
                    dy = bytebuf.get_i16_be()?;
                    dz = bytebuf.get_i16_be()?;
                }
                (Vector3::new(dx, dy, dz), on_ground)
            } else {
                // Linear VecDelta: 3 × i16
                let delta = Vector3::new(
                    bytebuf.get_i16_be()?,
                    bytebuf.get_i16_be()?,
                    bytebuf.get_i16_be()?,
                );
                (delta, on_ground)
            }
        } else if *version >= JavaMinecraftVersion::V_1_9 {
            let delta = Vector3::new(
                bytebuf.get_i16_be()?,
                bytebuf.get_i16_be()?,
                bytebuf.get_i16_be()?,
            );
            (delta, false) // onGround read below
        } else {
            let delta = Vector3::new(
                i16::from(bytebuf.get_i8()?) * 128,
                i16::from(bytebuf.get_i8()?) * 128,
                i16::from(bytebuf.get_i8()?) * 128,
            );
            (delta, false)
        };
        let yaw = bytebuf.get_u8()?;
        let pitch = bytebuf.get_u8()?;
        let on_ground = if *version >= JavaMinecraftVersion::V_26_3 {
            on_ground // already decoded from properties
        } else if *version >= JavaMinecraftVersion::V_1_8 {
            bytebuf.get_bool()?
        } else {
            false
        };
        Ok(Self {
            entity_id,
            delta,
            yaw,
            pitch,
            on_ground,
        })
    }
}
