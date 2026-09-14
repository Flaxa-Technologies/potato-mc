use crate::{
    ServerPacket,
    ser::{NetworkReadExt, ReadingError},
};
use pumpkin_data::packet::serverbound::play::MOVE_PLAYER_STATUS_ONLY;
use pumpkin_macros::java_packet;
use pumpkin_util::version::JavaMinecraftVersion;

#[java_packet(MOVE_PLAYER_STATUS_ONLY)]
pub struct SSetPlayerGround {
    /// Bit 0: on_ground, Bit 1: in_wall. Read as raw flags byte.
    pub on_ground: bool,
}

impl<'a> ServerPacket<'a> for SSetPlayerGround {
    fn read(bytebuf: &mut &'a [u8], _version: &JavaMinecraftVersion) -> Result<Self, ReadingError> {
        // Since 1.21.2 this byte is a flags field: bit 0 = on_ground, bit 1 = in_wall.
        // get_bool() would return true for any non-zero value (including 0x02 = wall-only),
        // incorrectly treating an in-wall-but-airborne player as on-ground.
        Ok(Self {
            on_ground: bytebuf.get_u8()? & 0x01 != 0,
        })
    }
}

impl crate::ClientPacket for SSetPlayerGround {
    fn write_packet_data(
        &self,
        mut write: impl std::io::Write,
        _version: &JavaMinecraftVersion,
    ) -> Result<(), crate::ser::WritingError> {
        use crate::ser::NetworkWriteExt;
        write.write_u8(self.on_ground as u8)?;
        Ok(())
    }
}
