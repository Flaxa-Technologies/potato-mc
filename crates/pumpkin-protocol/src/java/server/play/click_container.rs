use crate::VarInt;
use crate::codec::item_stack_seralizer::{ItemStackSerializer, OptionalItemStackHash};
use crate::{
    ServerPacket,
    ser::{NetworkReadExt, ReadingError},
};
use pumpkin_data::packet::serverbound::play::CONTAINER_CLICK;
use pumpkin_macros::java_packet;
use pumpkin_util::version::JavaMinecraftVersion;
use std::io::Read;

#[derive(Debug)]
#[java_packet(CONTAINER_CLICK)]
pub struct SClickSlot {
    pub sync_id: VarInt,
    pub revision: VarInt,
    pub slot: i16,
    pub button: i8,
    pub mode: SlotActionType,
    pub length_of_array: VarInt,
    pub array_of_changed_slots: Vec<(i16, OptionalItemStackHash)>,
    pub carried_item: OptionalItemStackHash,
}

impl SClickSlot {
    pub const BUTTON_LEFT: i8 = 0;
    pub const BUTTON_RIGHT: i8 = 1;
    pub const BUTTON_MIDDLE: i8 = 2;
    pub const BUTTON_DROP_SINGLE: i8 = 0;
    pub const BUTTON_DROP_STACK: i8 = 1;
    pub const BUTTON_OFFHAND_SWAP: i8 = 40;
}

impl<'a> ServerPacket<'a> for SClickSlot {
    fn read(
        mut bytebuf: &mut &'a [u8],
        version: &JavaMinecraftVersion,
    ) -> Result<Self, ReadingError> {
        let sync_id = bytebuf.get_container_id(version)?;
        let revision = if version >= &JavaMinecraftVersion::V_1_17_1 {
            bytebuf.get_var_int()?
        } else {
            VarInt(i32::from(bytebuf.get_i16_be()?))
        };
        let slot = bytebuf.get_i16_be()?;
        let button = bytebuf.get_i8()?;
        let mode = SlotActionType::read(&mut bytebuf)?;

        let length_of_array = bytebuf.get_var_int()?;
        if length_of_array.0 < 0 || length_of_array.0 > 256 {
            return Err(ReadingError::Message(
                "Changed slots length out of bounds".into(),
            ));
        }
        let mut array_of_changed_slots = Vec::with_capacity(length_of_array.0 as usize);
        let carried_item = if *version >= JavaMinecraftVersion::V_26_1 {
            for _ in 0..length_of_array.0 {
                array_of_changed_slots.push((
                    bytebuf.get_i16_be()?,
                    OptionalItemStackHash::read(&mut bytebuf)?,
                ));
            }
            OptionalItemStackHash::read(&mut bytebuf)?
        } else {
            for _ in 0..length_of_array.0 {
                let slot = bytebuf.get_i16_be()?;
                let item = ItemStackSerializer::read_with_version(&mut bytebuf, version)?;
                array_of_changed_slots
                    .push((slot, OptionalItemStackHash::from_stack(item.0.as_ref())));
            }
            let carried = ItemStackSerializer::read_with_version(&mut bytebuf, version)?;
            OptionalItemStackHash::from_stack(carried.0.as_ref())
        };

        Ok(Self {
            sync_id,
            revision,
            slot,
            button,
            mode,
            length_of_array,
            array_of_changed_slots,
            carried_item,
        })
    }
}

impl crate::ClientPacket for SClickSlot {
    fn write_packet_data(
        &self,
        mut write: impl std::io::Write,
        version: &JavaMinecraftVersion,
    ) -> Result<(), crate::ser::WritingError> {
        use crate::ser::NetworkWriteExt;
        write.write_container_id(&self.sync_id, version)?;
        if version >= &JavaMinecraftVersion::V_1_17_1 {
            write.write_var_int(&self.revision)?;
        } else {
            write.write_i16_be(self.revision.0 as i16)?;
        }
        write.write_i16_be(self.slot)?;
        write.write_i8(self.button)?;
        self.mode.write(&mut write)?;
        write.write_var_int(&VarInt(self.array_of_changed_slots.len() as i32))?;
        for (slot, item) in &self.array_of_changed_slots {
            write.write_i16_be(*slot)?;
            item.write(&mut write)?;
        }
        self.carried_item.write(&mut write)?;
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum SlotActionType {
    /// Performs a normal slot click. This can pick up or place items in the slot, possibly merging the cursor stack into the slot, or swapping the slot stack with the cursor stack if they can't be merged.
    Pickup,
    /// Performs a shift-click. This usually quickly moves items between the player's inventory and the open screen handler.
    QuickMove,
    /// Exchanges items between a slot and a hotbar slot. This is usually triggered by the player pressing a 1-9 number key while hovering over a slot.
    /// When the action type is swap, the click data is the hotbar slot to swap with (0-8).
    Swap,
    /// Clones the item in the slot. Usually triggered by middle clicking an item in creative mode.
    Clone,
    /// Throws the item out of the inventory. This is usually triggered by the player pressing Q while hovering over a slot, or clicking outside the window.
    /// When the action type is throw, the click data determines whether to throw a whole stack (1) or a single item from that stack (0).
    Throw,
    /// Drags items between multiple slots. This is usually triggered by the player clicking and dragging between slots.
    /// This action happens in 3 stages. Stage 0 signals that the drag has begun, and stage 2 signals that the drag has ended. In between multiple stage 1s signal which slots were dragged on.
    QuickCraft,
    /// Replenishes the cursor stack with items from the screen handler. This is usually triggered by the player double clicking.
    PickupAll,
}

impl SlotActionType {
    pub fn read(bytebuf: &mut impl Read) -> Result<Self, ReadingError> {
        let mode = bytebuf.get_var_int()?;
        Self::try_from(mode.0)
            .map_err(|_| ReadingError::Message("Invalid slot action type".to_string()))
    }

    pub fn write(
        &self,
        write: &mut impl crate::ser::NetworkWriteExt,
    ) -> Result<(), crate::ser::WritingError> {
        let mode = match self {
            Self::Pickup => 0,
            Self::QuickMove => 1,
            Self::Swap => 2,
            Self::Clone => 3,
            Self::Throw => 4,
            Self::QuickCraft => 5,
            Self::PickupAll => 6,
        };
        write.write_var_int(&VarInt(mode))
    }
}

#[derive(Debug)]
pub struct InvalidSlotActionType;

impl TryFrom<i32> for SlotActionType {
    type Error = InvalidSlotActionType;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Pickup),
            1 => Ok(Self::QuickMove),
            2 => Ok(Self::Swap),
            3 => Ok(Self::Clone),
            4 => Ok(Self::Throw),
            5 => Ok(Self::QuickCraft),
            6 => Ok(Self::PickupAll),
            _ => Err(InvalidSlotActionType),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ser::NetworkWriteExt;

    #[test]
    fn test_click_slot_empty_slots_1_21() {
        let mut buf = Vec::new();
        // sync_id (VarInt on 1.21)
        buf.write_var_int(&VarInt(1)).unwrap();
        // revision (VarInt on >= 1.17.1)
        buf.write_var_int(&VarInt(5)).unwrap();
        // slot (i16)
        buf.write_i16_be(36).unwrap();
        // button (i8)
        buf.write_i8(0).unwrap();
        // mode (VarInt)
        buf.write_var_int(&VarInt(0)).unwrap(); // Pickup
        // length_of_array (VarInt)
        buf.write_var_int(&VarInt(1)).unwrap();
        // changed slot 0: slot index = 36, empty item stack on 1.21 (item_count = 0)
        buf.write_i16_be(36).unwrap();
        buf.write_var_int(&VarInt(0)).unwrap(); // 0 count -> empty stack
        // carried item: empty stack on 1.21 (item_count = 0)
        buf.write_var_int(&VarInt(0)).unwrap();

        let mut slice = &buf[..];
        let packet = SClickSlot::read(&mut slice, &JavaMinecraftVersion::V_1_21).unwrap();
        assert_eq!(packet.sync_id.0, 1);
        assert_eq!(packet.revision.0, 5);
        assert_eq!(packet.slot, 36);
        assert_eq!(packet.button, 0);
        assert_eq!(packet.mode, SlotActionType::Pickup);
        assert_eq!(packet.array_of_changed_slots.len(), 1);
        assert_eq!(packet.array_of_changed_slots[0].0, 36);
        assert_eq!(packet.array_of_changed_slots[0].1, OptionalItemStackHash(None));
        assert_eq!(packet.carried_item, OptionalItemStackHash(None));
        assert!(slice.is_empty());
    }

    #[test]
    fn test_click_slot_empty_slots_26_1() {
        let mut buf = Vec::new();
        // sync_id (VarInt)
        buf.write_var_int(&VarInt(1)).unwrap();
        // revision (VarInt)
        buf.write_var_int(&VarInt(5)).unwrap();
        // slot (i16)
        buf.write_i16_be(36).unwrap();
        // button (i8)
        buf.write_i8(0).unwrap();
        // mode (VarInt)
        buf.write_var_int(&VarInt(0)).unwrap();
        // length_of_array (VarInt)
        buf.write_var_int(&VarInt(1)).unwrap();
        // changed slot 0: slot index = 36, empty OptionalItemStackHash (bool = false)
        buf.write_i16_be(36).unwrap();
        buf.write_bool(false).unwrap();
        // carried item: empty OptionalItemStackHash (bool = false)
        buf.write_bool(false).unwrap();

        let mut slice = &buf[..];
        let packet = SClickSlot::read(&mut slice, &JavaMinecraftVersion::V_26_1).unwrap();
        assert_eq!(packet.sync_id.0, 1);
        assert_eq!(packet.array_of_changed_slots[0].1, OptionalItemStackHash(None));
        assert_eq!(packet.carried_item, OptionalItemStackHash(None));
        assert!(slice.is_empty());
    }
}

