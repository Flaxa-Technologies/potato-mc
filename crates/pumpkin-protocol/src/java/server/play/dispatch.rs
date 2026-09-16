use pumpkin_data::packet::PacketId;
use pumpkin_data::packet::serverbound::play as play_packets;
use pumpkin_util::version::JavaMinecraftVersion;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ServerboundPlayPacketKind {
    ConfirmTeleport,
    ChangeGameMode,
    ChatAck,
    ChatCommand,
    ChatCommandSigned,
    ChatMessage,
    ClientInformation,
    ClientCommand,
    PlayerInput,
    MoveVehicle,
    PaddleBoat,
    Interact,
    BundleItemSelected,
    Attack,
    TeleportToEntity,
    KeepAlive,
    ClientTickEnd,
    TestInstanceBlockAction,
    SetTestBlock,
    DebugSubscriptionRequest,
    DebugSampleSubscription,
    PlayerPosition,
    PlayerPositionRotation,
    PlayerRotation,
    PlayerGround,
    PickItemFromBlock,
    PickItemFromEntity,
    PlayerAbilities,
    PlayerAction,
    SetCommandBlock,
    SetJigsawBlock,
    JigsawGenerate,
    PlayerCommand,
    PlayerLoaded,
    PlayPingRequest,
    ClickSlot,
    ContainerButtonClick,
    SetHeldItem,
    SetCreativeSlot,
    SwingArm,
    UpdateSign,
    EditBook,
    UseItemOn,
    UseItem,
    CommandSuggestion,
    CookieResponse,
    CloseContainer,
    ChunkBatch,
    PlayerSession,
    CustomPayload,
    RecipeBookChangeSettings,
    RecipeBookSeenRecipe,
    RenameItem,
    PlaceRecipe,
    CustomClickAction,
    SelectTrade,
    SeenAdvancement,
    PlayResourcePack,
    PlayPong,
    LockDifficulty,
    ChangeDifficulty,
    SetBeacon,
    ContainerSlotStateChanged,
    SpectateEntity,
    SetCommandMinecart,
    SetStructureBlock,
    SetGameRule,
    BlockEntityTagQuery,
    EntityTagQuery,
    ConfigurationAcknowledged,
}

const PAIRS: &[(PacketId, ServerboundPlayPacketKind)] = &[
    (play_packets::TELEPORT_CONFIRM, ServerboundPlayPacketKind::ConfirmTeleport),
    (play_packets::CHANGE_GAME_MODE, ServerboundPlayPacketKind::ChangeGameMode),
    (play_packets::CHAT_ACK, ServerboundPlayPacketKind::ChatAck),
    (play_packets::CHAT_COMMAND, ServerboundPlayPacketKind::ChatCommand),
    (play_packets::CHAT_COMMAND_SIGNED, ServerboundPlayPacketKind::ChatCommandSigned),
    (play_packets::CHAT_MESSAGE, ServerboundPlayPacketKind::ChatMessage),
    (play_packets::CLIENT_INFORMATION, ServerboundPlayPacketKind::ClientInformation),
    (play_packets::CLIENT_COMMAND, ServerboundPlayPacketKind::ClientCommand),
    (play_packets::PLAYER_INPUT, ServerboundPlayPacketKind::PlayerInput),
    (play_packets::MOVE_VEHICLE, ServerboundPlayPacketKind::MoveVehicle),
    (play_packets::PADDLE_BOAT, ServerboundPlayPacketKind::PaddleBoat),
    (play_packets::INTERACT, ServerboundPlayPacketKind::Interact),
    (play_packets::BUNDLE_ITEM_SELECTED, ServerboundPlayPacketKind::BundleItemSelected),
    (play_packets::ATTACK, ServerboundPlayPacketKind::Attack),
    (play_packets::TELEPORT_TO_ENTITY, ServerboundPlayPacketKind::TeleportToEntity),
    (play_packets::KEEP_ALIVE, ServerboundPlayPacketKind::KeepAlive),
    (play_packets::CLIENT_TICK_END, ServerboundPlayPacketKind::ClientTickEnd),
    (play_packets::TEST_INSTANCE_BLOCK_ACTION, ServerboundPlayPacketKind::TestInstanceBlockAction),
    (play_packets::SET_TEST_BLOCK, ServerboundPlayPacketKind::SetTestBlock),
    (play_packets::DEBUG_SUBSCRIPTION_REQUEST, ServerboundPlayPacketKind::DebugSubscriptionRequest),
    (play_packets::DEBUG_SAMPLE_SUBSCRIPTION, ServerboundPlayPacketKind::DebugSampleSubscription),
    (play_packets::MOVE_PLAYER_POS, ServerboundPlayPacketKind::PlayerPosition),
    (play_packets::MOVE_PLAYER_POS_ROT, ServerboundPlayPacketKind::PlayerPositionRotation),
    (play_packets::MOVE_PLAYER_ROT, ServerboundPlayPacketKind::PlayerRotation),
    (play_packets::MOVE_PLAYER_STATUS_ONLY, ServerboundPlayPacketKind::PlayerGround),
    (play_packets::PICK_ITEM_FROM_BLOCK, ServerboundPlayPacketKind::PickItemFromBlock),
    (play_packets::PICK_ITEM_FROM_ENTITY, ServerboundPlayPacketKind::PickItemFromEntity),
    (play_packets::PLAYER_ABILITIES, ServerboundPlayPacketKind::PlayerAbilities),
    (play_packets::PLAYER_ACTION, ServerboundPlayPacketKind::PlayerAction),
    (play_packets::SET_COMMAND_BLOCK, ServerboundPlayPacketKind::SetCommandBlock),
    (play_packets::SET_JIGSAW_BLOCK, ServerboundPlayPacketKind::SetJigsawBlock),
    (play_packets::JIGSAW_GENERATE, ServerboundPlayPacketKind::JigsawGenerate),
    (play_packets::PLAYER_COMMAND, ServerboundPlayPacketKind::PlayerCommand),
    (play_packets::PLAYER_LOADED, ServerboundPlayPacketKind::PlayerLoaded),
    (play_packets::PING_REQUEST, ServerboundPlayPacketKind::PlayPingRequest),
    (play_packets::CONTAINER_CLICK, ServerboundPlayPacketKind::ClickSlot),
    (play_packets::CONTAINER_BUTTON_CLICK, ServerboundPlayPacketKind::ContainerButtonClick),
    (play_packets::SET_CARRIED_ITEM, ServerboundPlayPacketKind::SetHeldItem),
    (play_packets::SET_CREATIVE_MODE_SLOT, ServerboundPlayPacketKind::SetCreativeSlot),
    (play_packets::SWING_ARM, ServerboundPlayPacketKind::SwingArm),
    (play_packets::SIGN_UPDATE, ServerboundPlayPacketKind::UpdateSign),
    (play_packets::EDIT_BOOK, ServerboundPlayPacketKind::EditBook),
    (play_packets::USE_ITEM_ON, ServerboundPlayPacketKind::UseItemOn),
    (play_packets::USE_ITEM, ServerboundPlayPacketKind::UseItem),
    (play_packets::COMMAND_SUGGESTION, ServerboundPlayPacketKind::CommandSuggestion),
    (play_packets::COOKIE_RESPONSE, ServerboundPlayPacketKind::CookieResponse),
    (play_packets::CONTAINER_CLOSE, ServerboundPlayPacketKind::CloseContainer),
    (play_packets::CHUNK_BATCH_RECEIVED, ServerboundPlayPacketKind::ChunkBatch),
    (play_packets::CHAT_SESSION_UPDATE, ServerboundPlayPacketKind::PlayerSession),
    (play_packets::CUSTOM_PAYLOAD, ServerboundPlayPacketKind::CustomPayload),
    (play_packets::RECIPE_BOOK_CHANGE_SETTINGS, ServerboundPlayPacketKind::RecipeBookChangeSettings),
    (play_packets::RECIPE_BOOK_SEEN_RECIPE, ServerboundPlayPacketKind::RecipeBookSeenRecipe),
    (play_packets::RENAME_ITEM, ServerboundPlayPacketKind::RenameItem),
    (play_packets::PLACE_RECIPE, ServerboundPlayPacketKind::PlaceRecipe),
    (play_packets::CUSTOM_CLICK_ACTION, ServerboundPlayPacketKind::CustomClickAction),
    (play_packets::SELECT_TRADE, ServerboundPlayPacketKind::SelectTrade),
    (play_packets::SEEN_ADVANCEMENTS, ServerboundPlayPacketKind::SeenAdvancement),
    (play_packets::RESOURCE_PACK, ServerboundPlayPacketKind::PlayResourcePack),
    (play_packets::PONG, ServerboundPlayPacketKind::PlayPong),
    (play_packets::LOCK_DIFFICULTY, ServerboundPlayPacketKind::LockDifficulty),
    (play_packets::CHANGE_DIFFICULTY, ServerboundPlayPacketKind::ChangeDifficulty),
    (play_packets::SET_BEACON, ServerboundPlayPacketKind::SetBeacon),
    (play_packets::CONTAINER_SLOT_STATE_CHANGED, ServerboundPlayPacketKind::ContainerSlotStateChanged),
    (play_packets::SPECTATE_ENTITY, ServerboundPlayPacketKind::SpectateEntity),
    (play_packets::SET_COMMAND_MINECART, ServerboundPlayPacketKind::SetCommandMinecart),
    (play_packets::SET_STRUCTURE_BLOCK, ServerboundPlayPacketKind::SetStructureBlock),
    (play_packets::SET_GAME_RULE, ServerboundPlayPacketKind::SetGameRule),
    (play_packets::BLOCK_ENTITY_TAG_QUERY, ServerboundPlayPacketKind::BlockEntityTagQuery),
    (play_packets::ENTITY_TAG_QUERY, ServerboundPlayPacketKind::EntityTagQuery),
    (play_packets::CONFIGURATION_ACKNOWLEDGED, ServerboundPlayPacketKind::ConfigurationAcknowledged),
];

pub const fn build_play_table(version: JavaMinecraftVersion) -> [Option<ServerboundPlayPacketKind>; 128] {
    let mut table = [None; 128];
    let mut i = 0;
    while i < PAIRS.len() {
        let pkt = PAIRS[i].0;
        let kind = PAIRS[i].1;
        let id = pkt.to_id(version);
        if id >= 0 && (id as usize) < 128 {
            table[id as usize] = Some(kind);
        }
        i += 1;
    }
    table
}

pub static PLAY_ID_MAP_1_21: [Option<ServerboundPlayPacketKind>; 128] = build_play_table(JavaMinecraftVersion::V_1_21);
pub static PLAY_ID_MAP_1_21_2: [Option<ServerboundPlayPacketKind>; 128] = build_play_table(JavaMinecraftVersion::V_1_21_2);
pub static PLAY_ID_MAP_1_21_4: [Option<ServerboundPlayPacketKind>; 128] = build_play_table(JavaMinecraftVersion::V_1_21_4);
pub static PLAY_ID_MAP_1_21_5: [Option<ServerboundPlayPacketKind>; 128] = build_play_table(JavaMinecraftVersion::V_1_21_5);
pub static PLAY_ID_MAP_1_21_6: [Option<ServerboundPlayPacketKind>; 128] = build_play_table(JavaMinecraftVersion::V_1_21_6);
pub static PLAY_ID_MAP_1_21_7: [Option<ServerboundPlayPacketKind>; 128] = build_play_table(JavaMinecraftVersion::V_1_21_7);
pub static PLAY_ID_MAP_1_21_9: [Option<ServerboundPlayPacketKind>; 128] = build_play_table(JavaMinecraftVersion::V_1_21_9);
pub static PLAY_ID_MAP_1_21_11: [Option<ServerboundPlayPacketKind>; 128] = build_play_table(JavaMinecraftVersion::V_1_21_11);
pub static PLAY_ID_MAP_26_1: [Option<ServerboundPlayPacketKind>; 128] = build_play_table(JavaMinecraftVersion::V_26_1);
pub static PLAY_ID_MAP_26_2: [Option<ServerboundPlayPacketKind>; 128] = build_play_table(JavaMinecraftVersion::V_26_2);

#[inline]
#[must_use]
pub fn resolve_play_packet_kind(
    packet_id: i32,
    version: &JavaMinecraftVersion,
) -> Option<ServerboundPlayPacketKind> {
    if packet_id < 0 || packet_id >= 128 {
        return None;
    }
    let idx = packet_id as usize;
    let table = match version {
        JavaMinecraftVersion::V_1_21 => &PLAY_ID_MAP_1_21,
        JavaMinecraftVersion::V_1_21_2 => &PLAY_ID_MAP_1_21_2,
        JavaMinecraftVersion::V_1_21_4 => &PLAY_ID_MAP_1_21_4,
        JavaMinecraftVersion::V_1_21_5 => &PLAY_ID_MAP_1_21_5,
        JavaMinecraftVersion::V_1_21_6 => &PLAY_ID_MAP_1_21_6,
        JavaMinecraftVersion::V_1_21_7 => &PLAY_ID_MAP_1_21_7,
        JavaMinecraftVersion::V_1_21_9 => &PLAY_ID_MAP_1_21_9,
        JavaMinecraftVersion::V_1_21_11 => &PLAY_ID_MAP_1_21_11,
        JavaMinecraftVersion::V_26_1 => &PLAY_ID_MAP_26_1,
        JavaMinecraftVersion::V_26_2 => &PLAY_ID_MAP_26_2,
        _ => return build_play_table(*version)[idx],
    };
    table[idx]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_play_packet_kind_target_versions() {
        for version in [
            JavaMinecraftVersion::V_1_21,
            JavaMinecraftVersion::V_1_21_2,
            JavaMinecraftVersion::V_1_21_4,
            JavaMinecraftVersion::V_26_1,
            JavaMinecraftVersion::V_26_2,
        ] {
            assert_eq!(
                resolve_play_packet_kind(0, &version),
                Some(ServerboundPlayPacketKind::ConfirmTeleport)
            );
            assert_eq!(resolve_play_packet_kind(-1, &version), None);
            assert_eq!(resolve_play_packet_kind(128, &version), None);
            assert_eq!(resolve_play_packet_kind(999, &version), None);
        }
    }

    #[test]
    fn test_attack_and_interact_routing_ids() {
        assert_eq!(
            resolve_play_packet_kind(22, &JavaMinecraftVersion::V_1_21),
            Some(ServerboundPlayPacketKind::Interact)
        );

        let attack_id_26_1 = play_packets::ATTACK.to_id(JavaMinecraftVersion::V_26_1);
        assert!(attack_id_26_1 >= 0);
        assert_eq!(
            resolve_play_packet_kind(attack_id_26_1, &JavaMinecraftVersion::V_26_1),
            Some(ServerboundPlayPacketKind::Attack)
        );

        let attack_id_26_2 = play_packets::ATTACK.to_id(JavaMinecraftVersion::V_26_2);
        assert!(attack_id_26_2 >= 0);
        assert_eq!(
            resolve_play_packet_kind(attack_id_26_2, &JavaMinecraftVersion::V_26_2),
            Some(ServerboundPlayPacketKind::Attack)
        );
    }

    #[test]
    fn test_container_click_ids() {
        assert_eq!(
            resolve_play_packet_kind(14, &JavaMinecraftVersion::V_1_21),
            Some(ServerboundPlayPacketKind::ClickSlot)
        );
        assert_eq!(
            resolve_play_packet_kind(16, &JavaMinecraftVersion::V_1_21_2),
            Some(ServerboundPlayPacketKind::ClickSlot)
        );
        assert_eq!(
            resolve_play_packet_kind(18, &JavaMinecraftVersion::V_26_1),
            Some(ServerboundPlayPacketKind::ClickSlot)
        );
        assert_eq!(
            resolve_play_packet_kind(18, &JavaMinecraftVersion::V_26_2),
            Some(ServerboundPlayPacketKind::ClickSlot)
        );
    }
}
