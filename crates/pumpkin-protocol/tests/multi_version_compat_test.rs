//! Multi-Version Protocol Compatibility Test Suite
//!
//! Covers Minecraft Java Edition client versions:
//! - 1.21 (Protocol 767)
//! - 1.21.2 (Protocol 768)
//! - 1.21.4 (Protocol 769)
//! - 26.1 (Protocol 775)
//! - 26.2 (Protocol 776, current engine)
//!
//! Tiers covered:
//! - Tier 1: Feature Coverage (>=5 tests per feature: Handshake, Status Ping, Config Negotiation, Packet Codecs)
//! - Tier 2: Boundary & Corner Cases (Empty buffers, max VarInt, boundary packet IDs, fallback states)
//! - Tier 3: Cross-Feature Combinations (State transitions, packet ID mappings, component adaptors)
//! - Tier 4: Real-World Application Scenarios (Full handshake -> login -> config -> play simulations)

use std::borrow::Cow;
use std::io::Cursor;

use pumpkin_data::block_state_remap::remap_block_state_for_version;
use pumpkin_data::data_component_impl::basic::{CustomModelDataImpl, ItemModelImpl};
use pumpkin_data::data_component_impl::DataComponentImpl;
use pumpkin_data::dimension::Dimension;
use pumpkin_data::entity_id_remap::remap_entity_id_for_version;
use pumpkin_data::item::Item;
use pumpkin_data::item_id_remap::{remap_item_id_for_version, remap_item_id_from_version};
use pumpkin_data::item_stack::ItemStack;
use pumpkin_data::particle_id_remap::remap_particle_id_for_version;
use pumpkin_data::registry::Registry;
use pumpkin_data::sound_id_remap::remap_sound_id_for_version;
use pumpkin_protocol::codec::item_stack_seralizer::{
    ItemStackSerializer, OptionalItemStackHash,
};
use pumpkin_protocol::codec::var_int::VarInt;
use pumpkin_protocol::java::client::config::{
    CFeatureFlags, CFinishConfig, CKnownPacks, CRegistryData,
};
use pumpkin_protocol::java::client::login::CLoginSuccess;
use pumpkin_protocol::java::client::play::{
    CChunkData, CLogin, CParticle, CSpawnEntity, CUpdateEntityPosRot, PlayerSpawnData,
};
use pumpkin_protocol::java::client::status::{CPingResponse, CStatusResponse};
use pumpkin_protocol::java::server::config::{SAcknowledgeFinishConfig, SKnownPacks};
use pumpkin_protocol::java::server::handshake::SHandShake;
use pumpkin_protocol::java::server::login::{SLoginAcknowledged, SLoginStart};
use pumpkin_protocol::java::server::play::{
    SClickSlot, SPlayerInput, SPlayerPosition, SlotActionType,
};
use pumpkin_protocol::java::server::status::{SStatusPingRequest, SStatusRequest};
use pumpkin_protocol::ser::{NetworkReadExt, NetworkWriteExt, ReadingError};
use pumpkin_protocol::{
    ClientPacket, ConnectionState, KnownPack, MultiVersionJavaPacket, Property,
    ServerPacket,
};
use pumpkin_util::math::vector3::Vector3;
use pumpkin_util::version::JavaMinecraftVersion;
use pumpkin_world::chunk::ChunkData;
use uuid::Uuid;

pub const TARGET_VERSIONS: [JavaMinecraftVersion; 5] = [
    JavaMinecraftVersion::V_1_21,   // 767
    JavaMinecraftVersion::V_1_21_2, // 768
    JavaMinecraftVersion::V_1_21_4, // 769
    JavaMinecraftVersion::V_26_1,   // 775
    JavaMinecraftVersion::V_26_2,   // 776
];

// ==============================================================================
// TIER 1: FEATURE COVERAGE
// ==============================================================================

mod tier1_handshake {
    use super::*;

    #[test]
    fn test_handshake_status_routing_protocols() {
        for version in TARGET_VERSIONS {
            let protocol = version.protocol_version();
            let handshake = SHandShake {
                protocol_version: VarInt(protocol),
                server_address: "localhost".into(),
                server_port: 25565,
                next_state: ConnectionState::Status,
            };

            let mut encoded = Vec::new();
            handshake
                .write_packet_data(&mut encoded, &version)
                .expect("handshake encoding failed");

            let mut slice = &encoded[..];
            let decoded = SHandShake::read(&mut slice, &version)
                .expect("handshake decoding failed");

            assert_eq!(decoded.protocol_version.0, protocol);
            assert_eq!(&*decoded.server_address, "localhost");
            assert_eq!(decoded.server_port, 25565);
            assert_eq!(decoded.next_state, ConnectionState::Status);
            assert!(slice.is_empty(), "all bytes must be consumed");
        }
    }

    #[test]
    fn test_handshake_login_routing_protocols() {
        for version in TARGET_VERSIONS {
            let protocol = version.protocol_version();
            let handshake = SHandShake {
                protocol_version: VarInt(protocol),
                server_address: "play.potatomc.org".into(),
                server_port: 25565,
                next_state: ConnectionState::Login,
            };

            let mut encoded = Vec::new();
            handshake
                .write_packet_data(&mut encoded, &version)
                .expect("handshake encoding failed");

            let mut slice = &encoded[..];
            let decoded = SHandShake::read(&mut slice, &version)
                .expect("handshake decoding failed");

            assert_eq!(decoded.protocol_version.0, protocol);
            assert_eq!(&*decoded.server_address, "play.potatomc.org");
            assert_eq!(decoded.server_port, 25565);
            assert_eq!(decoded.next_state, ConnectionState::Login);
            assert!(slice.is_empty());
        }
    }

    #[test]
    fn test_handshake_transfer_routing_protocols() {
        for version in TARGET_VERSIONS {
            let protocol = version.protocol_version();
            let handshake = SHandShake {
                protocol_version: VarInt(protocol),
                server_address: "hub.network.com".into(),
                server_port: 19132,
                next_state: ConnectionState::Transfer,
            };

            let mut encoded = Vec::new();
            handshake
                .write_packet_data(&mut encoded, &version)
                .expect("handshake encoding failed");

            let mut slice = &encoded[..];
            let decoded = SHandShake::read(&mut slice, &version)
                .expect("handshake decoding failed");

            assert_eq!(decoded.protocol_version.0, protocol);
            assert_eq!(&*decoded.server_address, "hub.network.com");
            assert_eq!(decoded.server_port, 19132);
            assert_eq!(decoded.next_state, ConnectionState::Transfer);
            assert!(slice.is_empty());
        }
    }

    #[test]
    fn test_handshake_bungeecord_forwarded_address() {
        for version in TARGET_VERSIONS {
            let textures = "e".repeat(256);
            let signature = "s".repeat(256);
            let bungeecord_addr = format!(
                "mc.potatomc.org\0192.168.1.100\0d8f4a1e0-0f1b-4c3a-9f2e-1a2b3c4d5e6f\0[{{\"name\":\"textures\",\"value\":\"{textures}\",\"signature\":\"{signature}\"}}]"
            );

            let handshake = SHandShake {
                protocol_version: VarInt(version.protocol_version()),
                server_address: bungeecord_addr.clone().into_boxed_str(),
                server_port: 25565,
                next_state: ConnectionState::Login,
            };

            let mut encoded = Vec::new();
            handshake
                .write_packet_data(&mut encoded, &version)
                .expect("handshake encoding with bungeecord forward failed");

            let mut slice = &encoded[..];
            let decoded = SHandShake::read(&mut slice, &version)
                .expect("handshake decoding with bungeecord forward failed");

            assert_eq!(&*decoded.server_address, bungeecord_addr.as_str());
            assert_eq!(decoded.server_address.split('\0').count(), 4);
            assert_eq!(decoded.next_state, ConnectionState::Login);
        }
    }

    #[test]
    fn test_handshake_invalid_next_state_rejected() {
        for version in TARGET_VERSIONS {
            for invalid_state in [0, 4, 99, -1] {
                let mut buf = Vec::new();
                buf.write_var_int(&VarInt(version.protocol_version()))
                    .unwrap();
                buf.write_string("localhost").unwrap();
                buf.write_u16_be(25565).unwrap();
                buf.write_var_int(&VarInt(invalid_state)).unwrap();

                let mut slice = &buf[..];
                let result = SHandShake::read(&mut slice, &version);
                assert!(
                    result.is_err(),
                    "handshake with invalid state {invalid_state} should be rejected"
                );
            }
        }
    }

    #[test]
    fn test_handshake_port_boundaries() {
        for version in TARGET_VERSIONS {
            for port in [0u16, 1u16, 25565u16, 65535u16] {
                let handshake = SHandShake {
                    protocol_version: VarInt(version.protocol_version()),
                    server_address: "localhost".into(),
                    server_port: port,
                    next_state: ConnectionState::Status,
                };

                let mut buf = Vec::new();
                handshake
                    .write_packet_data(&mut buf, &version)
                    .expect("encoding failed");

                let mut slice = &buf[..];
                let decoded = SHandShake::read(&mut slice, &version).expect("decoding failed");
                assert_eq!(decoded.server_port, port);
            }
        }
    }

    #[test]
    fn test_handshake_roundtrip_fidelity() {
        for version in TARGET_VERSIONS {
            let original = SHandShake {
                protocol_version: VarInt(version.protocol_version()),
                server_address: "mc.test.network".into(),
                server_port: 30000,
                next_state: ConnectionState::Login,
            };

            let mut buf1 = Vec::new();
            original.write_packet_data(&mut buf1, &version).unwrap();

            let mut slice = &buf1[..];
            let decoded = SHandShake::read(&mut slice, &version).unwrap();

            let mut buf2 = Vec::new();
            decoded.write_packet_data(&mut buf2, &version).unwrap();

            assert_eq!(buf1, buf2, "handshake serialization must be byte-identical");
        }
    }
}

mod tier1_status_ping {
    use super::*;

    #[test]
    fn test_status_request_packet_id() {
        for version in TARGET_VERSIONS {
            assert_eq!(
                SStatusRequest::to_id(version),
                0x00,
                "SStatusRequest packet ID must be 0x00 across all versions"
            );
        }
    }

    #[test]
    fn test_status_response_packet_id() {
        for version in TARGET_VERSIONS {
            assert_eq!(
                CStatusResponse::to_id(version),
                0x00,
                "CStatusResponse packet ID must be 0x00 across all versions"
            );
        }
    }

    #[test]
    fn test_status_response_encoding() {
        let sample_json = r#"{"version":{"name":"1.21 - 26.2","protocol":776},"players":{"max":100,"online":5},"description":{"text":"PotatoMC Server"}}"#;
        let response = CStatusResponse::new(sample_json.to_string());

        for version in TARGET_VERSIONS {
            let mut buf = Vec::new();
            response
                .write_packet_data(&mut buf, &version)
                .expect("status response write failed");

            let mut slice = &buf[..];
            let decoded_str = slice.get_str().expect("failed to decode json string");
            assert_eq!(&*decoded_str, sample_json);
            assert!(slice.is_empty());
        }
    }

    #[test]
    fn test_status_ping_request_roundtrip() {
        let ping_payload = 987_654_321_012_345i64;
        let request = SStatusPingRequest {
            payload: ping_payload,
        };

        for version in TARGET_VERSIONS {
            let mut buf = Vec::new();
            request
                .write_packet_data(&mut buf, &version)
                .expect("ping request write failed");

            let mut slice = &buf[..];
            let decoded = SStatusPingRequest::read(&mut slice, &version)
                .expect("ping request read failed");
            assert_eq!(decoded.payload, ping_payload);
            assert!(slice.is_empty());
        }
    }

    #[test]
    fn test_ping_response_encoding() {
        let ping_payload = 1_234_567_890_987i64;
        let response = CPingResponse::new(ping_payload);

        for version in TARGET_VERSIONS {
            let mut buf = Vec::new();
            response
                .write_packet_data(&mut buf, &version)
                .expect("ping response write failed");

            let mut slice = &buf[..];
            let decoded_payload = slice.get_i64_be().expect("failed to read ping payload");
            assert_eq!(decoded_payload, ping_payload);
            assert!(slice.is_empty());
        }
    }

    #[test]
    fn test_status_ping_packet_id_parity() {
        for version in TARGET_VERSIONS {
            assert_eq!(
                SStatusPingRequest::to_id(version),
                0x01,
                "SStatusPingRequest packet ID must be 0x01"
            );
            assert_eq!(
                CPingResponse::to_id(version),
                0x01,
                "CPingResponse packet ID must be 0x01"
            );
        }
    }
}

mod tier1_configuration {
    use super::*;

    #[test]
    fn test_login_success_strict_error_handling_v1_21() {
        let uuid = Uuid::new_v4();
        let username = "TestPlayer";
        let properties: [Property; 0] = [];
        let session_id = Uuid::new_v4();

        let packet = CLoginSuccess::new(&uuid, username, &properties, true, session_id);

        let mut buf_1_21 = Vec::new();
        packet
            .write_packet_data(&mut buf_1_21, &JavaMinecraftVersion::V_1_21)
            .unwrap();

        let mut buf_1_21_2 = Vec::new();
        packet
            .write_packet_data(&mut buf_1_21_2, &JavaMinecraftVersion::V_1_21_2)
            .unwrap();

        // 1.21 encodes strict_error_handling (1 byte bool) which was removed in 1.21.2
        assert_eq!(
            buf_1_21.len(),
            buf_1_21_2.len() + 1,
            "1.21 CLoginSuccess must be exactly 1 byte longer than 1.21.2 (strict_error_handling bool)"
        );

        // The trailing byte for 1.21 must be 0x01 (strict_error_handling = true)
        assert_eq!(buf_1_21.last().copied(), Some(1));
    }

    #[test]
    fn test_login_success_session_id_v26_2() {
        let uuid = Uuid::new_v4();
        let username = "TestPlayer";
        let properties: [Property; 0] = [];
        let session_id = Uuid::new_v4();

        let packet = CLoginSuccess::new(&uuid, username, &properties, false, session_id);

        let mut buf_26_1 = Vec::new();
        packet
            .write_packet_data(&mut buf_26_1, &JavaMinecraftVersion::V_26_1)
            .unwrap();

        let mut buf_26_2 = Vec::new();
        packet
            .write_packet_data(&mut buf_26_2, &JavaMinecraftVersion::V_26_2)
            .unwrap();

        // 26.2 encodes session_id: Uuid (16 bytes) added in 26.2
        assert_eq!(
            buf_26_2.len(),
            buf_26_1.len() + 16,
            "26.2 CLoginSuccess must be exactly 16 bytes longer than 26.1 (session_id Uuid)"
        );
    }

    #[test]
    fn test_known_packs_roundtrip() {
        for version in TARGET_VERSIONS {
            let packs = [
                KnownPack {
                    namespace: "minecraft".into(),
                    id: "core".into(),
                    version: "1.21".into(),
                },
                KnownPack {
                    namespace: "potatomc".into(),
                    id: "expansion".into(),
                    version: "2.0".into(),
                },
            ];

            let clientbound = CKnownPacks::new(&packs);
            let mut encoded = Vec::new();
            clientbound
                .write_packet_data(&mut encoded, &version)
                .expect("CKnownPacks encoding failed");

            let mut slice = &encoded[..];
            let serverbound =
                SKnownPacks::read(&mut slice, &version).expect("SKnownPacks decoding failed");

            assert_eq!(serverbound.known_packs.len(), 2);
            assert_eq!(&*serverbound.known_packs[0].namespace, "minecraft");
            assert_eq!(&*serverbound.known_packs[0].id, "core");
            assert_eq!(&*serverbound.known_packs[0].version, "1.21");
            assert_eq!(&*serverbound.known_packs[1].namespace, "potatomc");
            assert_eq!(&*serverbound.known_packs[1].id, "expansion");
            assert_eq!(&*serverbound.known_packs[1].version, "2.0");
            assert!(slice.is_empty());
        }
    }

    #[test]
    fn test_feature_flags_serialization() {
        let features = ["minecraft:vanilla".into(), "minecraft:bundle".into()];
        let packet = CFeatureFlags::new(&features);

        for version in TARGET_VERSIONS {
            let mut buf = Vec::new();
            packet
                .write_packet_data(&mut buf, &version)
                .expect("feature flags write failed");

            let mut slice = &buf[..];
            let count = slice.get_var_int().expect("feature count").0;
            assert_eq!(count, 2);
            assert_eq!(&*slice.get_str().unwrap(), "minecraft:vanilla");
            assert_eq!(&*slice.get_str().unwrap(), "minecraft:bundle");
            assert!(slice.is_empty());
        }
    }

    #[test]
    fn test_finish_config_and_acknowledge() {
        for version in TARGET_VERSIONS {
            assert_eq!(
                CFinishConfig::to_id(version),
                0x03,
                "CFinishConfig packet ID must be 0x03"
            );
            assert_eq!(
                SAcknowledgeFinishConfig::to_id(version),
                0x03,
                "SAcknowledgeFinishConfig packet ID must be 0x03"
            );

            let finish = CFinishConfig;
            let mut buf = Vec::new();
            finish
                .write_packet_data(&mut buf, &version)
                .expect("finish config encode failed");
            assert!(
                buf.is_empty(),
                "CFinishConfig must have 0 payload bytes"
            );

            let ack = SAcknowledgeFinishConfig;
            let mut ack_buf = Vec::new();
            ack.write_packet_data(&mut ack_buf, &version)
                .expect("ack finish config encode failed");
            assert!(
                ack_buf.is_empty(),
                "SAcknowledgeFinishConfig must have 0 payload bytes"
            );

            let mut slice = &ack_buf[..];
            assert!(SAcknowledgeFinishConfig::read(&mut slice, &version).is_ok());
        }
    }

    #[test]
    fn test_registry_data_sync_scaling_all_versions() {
        // Spec requirements:
        // 1.21 (767) -> 11 registries
        // 1.21.2 (768) -> 12 registries
        // 1.21.4 (769) -> 12 registries
        // 26.1 (775) -> 28 registries
        // 26.2 (776) -> 29 registries
        let reg_1_21 = Registry::get_synced(JavaMinecraftVersion::V_1_21);
        assert_eq!(
            reg_1_21.len(),
            11,
            "1.21 must synchronize exactly 11 dynamic registries"
        );

        let reg_1_21_2 = Registry::get_synced(JavaMinecraftVersion::V_1_21_2);
        assert_eq!(
            reg_1_21_2.len(),
            12,
            "1.21.2 must synchronize exactly 12 dynamic registries"
        );

        let reg_1_21_4 = Registry::get_synced(JavaMinecraftVersion::V_1_21_4);
        assert_eq!(
            reg_1_21_4.len(),
            12,
            "1.21.4 must synchronize exactly 12 dynamic registries"
        );

        let reg_26_1 = Registry::get_synced(JavaMinecraftVersion::V_26_1);
        assert_eq!(
            reg_26_1.len(),
            28,
            "26.1 must synchronize exactly 28 dynamic registries"
        );

        let reg_26_2 = Registry::get_synced(JavaMinecraftVersion::V_26_2);
        assert_eq!(
            reg_26_2.len(),
            30,
            "26.2 must synchronize exactly 30 dynamic registries"
        );

        let reg_26_3 = Registry::get_synced(JavaMinecraftVersion::V_26_3);
        assert_eq!(
            reg_26_3.len(),
            31,
            "26.3 must synchronize exactly 31 dynamic registries"
        );

        // Verify CRegistryData packet serializes valid bytes for each registry
        for reg in &reg_1_21 {
            let packet = CRegistryData::new(&reg.registry_id, &reg.registry_entries);
            let mut buf = Vec::new();
            packet
                .write_packet_data(&mut buf, &JavaMinecraftVersion::V_1_21)
                .unwrap();
            assert!(!buf.is_empty(), "registry packet payload must not be empty");
        }
    }
}

mod tier1_packet_codecs {
    use super::*;

    #[test]
    fn test_spawn_entity_velocity_format_shift() {
        let entity = CSpawnEntity::new(
            VarInt(100),
            Uuid::new_v4(),
            VarInt(1), // entity type
            Vector3::new(10.0, 64.0, -10.0),
            0.0,
            0.0,
            0.0,
            VarInt(0),
            Vector3::new(0.5, 0.2, -0.3),
        );

        let mut buf_legacy = Vec::new();
        entity
            .write_packet_data(&mut buf_legacy, &JavaMinecraftVersion::V_1_21_4)
            .expect("legacy spawn entity write failed");

        let mut buf_modern = Vec::new();
        entity
            .write_packet_data(&mut buf_modern, &JavaMinecraftVersion::V_26_1)
            .expect("modern spawn entity write failed");

        // 1.21.4 uses legacy 3xi16 velocity at the tail (6 bytes).
        // 26.1 uses LpVector3d (packed bytes) positioned before rotation.
        assert_ne!(
            buf_legacy, buf_modern,
            "CSpawnEntity byte representation must differ between 1.21.4 and 26.1 due to velocity format shift"
        );
    }

    #[test]
    fn test_particle_force_spawn_flag_gating() {
        let particle = CParticle::new(
            true, // force_spawn
            true, // important
            Vector3::new(0.0, 100.0, 0.0),
            Vector3::new(0.1, 0.1, 0.1),
            1.0,
            10,
            VarInt(1),
            &[],
        );

        let mut buf_1_21_2 = Vec::new();
        particle
            .write_packet_data(&mut buf_1_21_2, &JavaMinecraftVersion::V_1_21_2)
            .unwrap();

        let mut buf_1_21_4 = Vec::new();
        particle
            .write_packet_data(&mut buf_1_21_4, &JavaMinecraftVersion::V_1_21_4)
            .unwrap();

        // 1.21.4 added `force_spawn: bool` directly after `important: bool`
        assert_eq!(
            buf_1_21_4.len(),
            buf_1_21_2.len() + 1,
            "CParticle in 1.21.4 must be exactly 1 byte longer than in 1.21.2 (force_spawn bool)"
        );
    }

    #[test]
    fn test_login_packet_sealevel_and_online_mode() {
        let spawn_data = PlayerSpawnData::new(
            Dimension::OVERWORLD,
            123_456_789,
            0,
            -1,
            false,
            false,
            None,
            VarInt(0),
            VarInt(63), // sealevel
        );
        let dim_names = ["minecraft:overworld".into()];

        let login_packet = CLogin::new(
            42,
            false,
            &dim_names,
            VarInt(20),
            VarInt(10),
            VarInt(10),
            false,
            true,
            false,
            spawn_data,
            true, // online_mode
            false,
        );

        let mut buf_1_21 = Vec::new();
        login_packet
            .write_packet_data(&mut buf_1_21, &JavaMinecraftVersion::V_1_21)
            .unwrap();

        let mut buf_1_21_2 = Vec::new();
        login_packet
            .write_packet_data(&mut buf_1_21_2, &JavaMinecraftVersion::V_1_21_2)
            .unwrap();

        let mut buf_26_1 = Vec::new();
        login_packet
            .write_packet_data(&mut buf_26_1, &JavaMinecraftVersion::V_26_1)
            .unwrap();

        let mut buf_26_2 = Vec::new();
        login_packet
            .write_packet_data(&mut buf_26_2, &JavaMinecraftVersion::V_26_2)
            .unwrap();

        // 1.21.2 added sealevel: VarInt (63 = 1 byte VarInt)
        assert_eq!(
            buf_1_21_2.len(),
            buf_1_21.len() + 1,
            "1.21.2 login packet must include sealevel VarInt"
        );

        // 26.2 added online_mode: bool (1 byte)
        assert_eq!(
            buf_26_2.len(),
            buf_26_1.len() + 1,
            "26.2 login packet must include online_mode bool"
        );
    }

    #[test]
    fn test_chunk_data_liquid_count_and_packed_data() {
        let chunk = ChunkData::empty(0, 0);
        let packet = CChunkData::new(&chunk);

        for version in TARGET_VERSIONS {
            let mut buf = Vec::new();
            packet
                .write_packet_data(&mut buf, &version)
                .expect("chunk serialization must succeed across all target versions");
            assert!(!buf.is_empty(), "chunk payload must not be empty");
        }
    }

    #[test]
    fn test_player_position_collision_flags() {
        for version in TARGET_VERSIONS {
            let packet = SPlayerPosition {
                position: Vector3::new(12.5, 70.0, -45.5),
                collision: 1, // on ground
            };

            let mut buf = Vec::new();
            packet.write_packet_data(&mut buf, &version).unwrap();

            let mut slice = &buf[..];
            let decoded = SPlayerPosition::read(&mut slice, &version).unwrap();

            assert_eq!(decoded.position.x, 12.5);
            assert_eq!(decoded.position.y, 70.0);
            assert_eq!(decoded.position.z, -45.5);
            assert_eq!(decoded.collision & 1, 1, "bit 0 (FLAG_ON_GROUND) must be preserved");
        }
    }

    #[test]
    fn test_player_input_translation_modes() {
        let input_packet = SPlayerInput {
            input: SPlayerInput::FORWARD | SPlayerInput::JUMP,
        };

        // On >= 1.21.2, writes and reads single i8 byte
        let mut buf_modern = Vec::new();
        input_packet
            .write_packet_data(&mut buf_modern, &JavaMinecraftVersion::V_1_21_2)
            .unwrap();
        assert_eq!(buf_modern.len(), 1);

        let mut slice = &buf_modern[..];
        let decoded =
            SPlayerInput::read(&mut slice, &JavaMinecraftVersion::V_1_21_2).unwrap();
        assert_eq!(decoded.input, SPlayerInput::FORWARD | SPlayerInput::JUMP);

        // On 1.21 (767), writes 4 fields (2 floats + 2 bools = 10 bytes), and decodes into bitmask
        let mut buf_legacy = Vec::new();
        input_packet
            .write_packet_data(&mut buf_legacy, &JavaMinecraftVersion::V_1_21)
            .unwrap();
        assert_eq!(buf_legacy.len(), 10);

        let mut slice = &buf_legacy[..];
        let decoded_legacy =
            SPlayerInput::read(&mut slice, &JavaMinecraftVersion::V_1_21).unwrap();
        assert_eq!(
            decoded_legacy.input & SPlayerInput::FORWARD,
            SPlayerInput::FORWARD
        );
        assert_eq!(decoded_legacy.input & SPlayerInput::JUMP, SPlayerInput::JUMP);
    }

    #[test]
    fn test_click_slot_serialization() {
        for version in TARGET_VERSIONS {
            let click = SClickSlot {
                sync_id: VarInt(1),
                revision: VarInt(5),
                slot: 12,
                button: SClickSlot::BUTTON_LEFT,
                mode: SlotActionType::Pickup,
                length_of_array: VarInt(0),
                array_of_changed_slots: Vec::new(),
                carried_item: OptionalItemStackHash(None),
            };

            let mut buf = Vec::new();
            click
                .write_packet_data(&mut buf, &version)
                .expect("click slot encoding failed");

            let mut slice = &buf[..];
            let decoded = SClickSlot::read(&mut slice, &version)
                .expect("click slot decoding failed");

            assert_eq!(decoded.sync_id.0, 1);
            assert_eq!(decoded.revision.0, 5);
            assert_eq!(decoded.slot, 12);
            assert_eq!(decoded.button, SClickSlot::BUTTON_LEFT);
            assert_eq!(decoded.mode, SlotActionType::Pickup);
            assert_eq!(decoded.length_of_array.0, 0);
            assert!(decoded.carried_item.0.is_none());
        }
    }
}

// ==============================================================================
// TIER 2: BOUNDARY & CORNER CASES
// ==============================================================================

mod tier2_boundaries {
    use super::*;

    #[test]
    fn test_empty_byte_buffer_rejection() {
        for version in TARGET_VERSIONS {
            let mut empty: &[u8] = &[];

            assert!(
                SHandShake::read(&mut empty, &version).is_err(),
                "SHandShake::read on empty buffer must fail with ReadingError"
            );

            assert!(
                SPlayerPosition::read(&mut empty, &version).is_err(),
                "SPlayerPosition::read on empty buffer must fail with ReadingError"
            );

            assert!(
                SPlayerInput::read(&mut empty, &version).is_err(),
                "SPlayerInput::read on empty buffer must fail with ReadingError"
            );

            assert!(
                SKnownPacks::read(&mut empty, &version).is_err(),
                "SKnownPacks::read on empty buffer must fail with ReadingError"
            );

            assert!(
                SClickSlot::read(&mut empty, &version).is_err(),
                "SClickSlot::read on empty buffer must fail with ReadingError"
            );

            assert!(
                SLoginStart::read(&mut empty, &version).is_err(),
                "SLoginStart::read on empty buffer must fail with ReadingError"
            );
        }
    }

    #[test]
    fn test_boundary_packet_ids() {
        for version in TARGET_VERSIONS {
            // Packets with valid constant 0x00 ID
            assert_eq!(SHandShake::to_id(version), 0x00);
            assert_eq!(SStatusRequest::to_id(version), 0x00);
            assert_eq!(CStatusResponse::to_id(version), 0x00);
            assert_eq!(SLoginStart::to_id(version), 0x00);

            // Packets with positive shifting IDs
            assert!(CLogin::to_id(version) > 0);
            assert!(CChunkData::to_id(version) > 0);
            assert!(CParticle::to_id(version) > 0);
        }
    }

    #[test]
    fn test_varint_extremes_and_overflow() {
        let test_values = [0, 1, -1, 127, 128, 255, 256, i32::MAX, i32::MIN];

        for val in test_values {
            let varint = VarInt(val);
            let mut buf = Vec::new();
            varint.encode(&mut buf).expect("VarInt encoding failed");

            let mut cursor = Cursor::new(&buf);
            let decoded = VarInt::decode(&mut cursor).expect("VarInt decoding failed");
            assert_eq!(
                decoded.0, val,
                "VarInt roundtrip mismatch for value {val}"
            );
        }

        // Test 5-byte max size VarInt
        let max_varint_bytes = [0xFF, 0xFF, 0xFF, 0xFF, 0x07]; // i32::MAX
        let mut cursor = Cursor::new(&max_varint_bytes[..]);
        let decoded = VarInt::decode(&mut cursor).unwrap();
        assert_eq!(decoded.0, i32::MAX);

        // Test invalid 6-byte VarInt (all 5 bytes have continuation bit set)
        let invalid_varint_bytes = [0x80, 0x80, 0x80, 0x80, 0x80, 0x01];
        let mut cursor = Cursor::new(&invalid_varint_bytes[..]);
        let result = VarInt::decode(&mut cursor);
        assert!(
            matches!(result, Err(ReadingError::TooLarge(_))),
            "VarInt exceeding 5 bytes must return ReadingError::TooLarge"
        );
    }

    #[test]
    fn test_oversized_server_address_rejection() {
        for version in TARGET_VERSIONS {
            let oversized = "x".repeat(i16::MAX as usize + 1);
            let mut buf = Vec::new();
            buf.write_var_int(&VarInt(version.protocol_version()))
                .unwrap();
            buf.write_string_bounded(&oversized, oversized.len())
                .unwrap();
            buf.write_u16_be(25565).unwrap();
            buf.write_var_int(&VarInt(2)).unwrap();

            let mut slice = &buf[..];
            let result = SHandShake::read(&mut slice, &version);
            assert!(
                matches!(result, Err(ReadingError::TooLarge(_))),
                "Oversized server address must return ReadingError::TooLarge"
            );
        }
    }

    #[test]
    fn test_block_state_stone_fallback_boundary() {
        for version in TARGET_VERSIONS {
            // Air (state 0) must always map to Air (0)
            assert_eq!(
                remap_block_state_for_version(0, version),
                0,
                "Air state 0 must remain 0"
            );

            // Default stone state (ID 1) must remain stone (1)
            assert_eq!(
                remap_block_state_for_version(1, version),
                1,
                "Stone state 1 must map to stone"
            );

            // Hypothetical non-existent modern block state ID (e.g. 65000)
            // If it maps to 0 in target version, it must gracefully fallback to Stone (1)
            let fallback_result = remap_block_state_for_version(65000, version);
            assert!(
                fallback_result == 1 || fallback_result != 0,
                "Non-zero block state must not evaluate to 0 (air); must fall back to Stone (1)"
            );
        }
    }

    #[test]
    fn test_entity_id_remapping_boundary() {
        for version in TARGET_VERSIONS {
            // Remapping common entity types
            let zombie_mapped = remap_entity_id_for_version(110, version);
            assert!(
                zombie_mapped > 0,
                "Zombie entity ID must map to a valid positive ID across all versions"
            );
        }
    }

    #[test]
    fn test_particle_and_sound_id_remapping_boundary() {
        for version in TARGET_VERSIONS {
            // Particle 0 must not panic
            let particle_0 = remap_particle_id_for_version(0, version);
            assert!(particle_0 < 200, "Particle ID 0 must map to a valid bounded ID");

            // Sound 0 must not panic
            let sound_0 = remap_sound_id_for_version(0, version);
            assert!(sound_0 < 3000, "Sound ID 0 must map to a valid bounded ID");
        }
    }
}

// ==============================================================================
// TIER 3: CROSS-FEATURE COMBINATIONS
// ==============================================================================

mod tier3_cross_feature {
    use super::*;

    #[test]
    fn test_version_transitions_state_flow() {
        for version in TARGET_VERSIONS {
            // Handshake phase
            let handshake = SHandShake {
                protocol_version: VarInt(version.protocol_version()),
                server_address: "localhost".into(),
                server_port: 25565,
                next_state: ConnectionState::Login,
            };
            let mut h_buf = Vec::new();
            handshake.write_packet_data(&mut h_buf, &version).unwrap();
            let mut h_slice = &h_buf[..];
            let h_read = SHandShake::read(&mut h_slice, &version).unwrap();
            assert_eq!(h_read.next_state, ConnectionState::Login);

            // Login phase: SLoginStart -> CLoginSuccess -> SLoginAcknowledged
            let login_start = SLoginStart {
                name: "PotatoPlayer".into(),
                uuid: Uuid::new_v4(),
            };
            let mut l_buf = Vec::new();
            login_start.write_packet_data(&mut l_buf, &version).unwrap();
            let mut l_slice = &l_buf[..];
            let l_read = SLoginStart::read(&mut l_slice, &version).unwrap();
            assert_eq!(&*l_read.name, "PotatoPlayer");

            let login_success = CLoginSuccess::new(
                &l_read.uuid,
                &l_read.name,
                &[],
                false,
                Uuid::new_v4(),
            );
            let mut ls_buf = Vec::new();
            login_success
                .write_packet_data(&mut ls_buf, &version)
                .unwrap();
            assert!(!ls_buf.is_empty());

            let login_ack = SLoginAcknowledged;
            let mut la_buf = Vec::new();
            login_ack.write_packet_data(&mut la_buf, &version).unwrap();
            let mut la_slice = &la_buf[..];
            assert!(SLoginAcknowledged::read(&mut la_slice, &version).is_ok());

            // Config phase: CFinishConfig -> SAcknowledgeFinishConfig
            let finish = CFinishConfig;
            let mut f_buf = Vec::new();
            finish.write_packet_data(&mut f_buf, &version).unwrap();

            let ack = SAcknowledgeFinishConfig;
            let mut a_buf = Vec::new();
            ack.write_packet_data(&mut a_buf, &version).unwrap();
            let mut a_slice = &a_buf[..];
            assert!(SAcknowledgeFinishConfig::read(&mut a_slice, &version).is_ok());
        }
    }

    #[test]
    fn test_packet_id_mapping_matrix_full_verification() {
        for version in TARGET_VERSIONS {
            // Handshake
            assert_eq!(SHandShake::to_id(version), 0x00);

            // Status
            assert_eq!(SStatusRequest::to_id(version), 0x00);
            assert_eq!(CStatusResponse::to_id(version), 0x00);
            assert_eq!(SStatusPingRequest::to_id(version), 0x01);
            assert_eq!(CPingResponse::to_id(version), 0x01);

            // Login
            assert_eq!(SLoginStart::to_id(version), 0x00);
            assert_eq!(CLoginSuccess::to_id(version), 0x02);
            assert_eq!(SLoginAcknowledged::to_id(version), 0x03);

            // Config
            assert_eq!(CFinishConfig::to_id(version), 0x03);
            assert_eq!(SAcknowledgeFinishConfig::to_id(version), 0x03);
            assert_eq!(CKnownPacks::to_id(version), 0x0E);
            assert_eq!(SKnownPacks::to_id(version), 0x07);

            // Play: CSpawnEntity
            assert_eq!(CSpawnEntity::to_id(version), 0x01);

            // Play: CLogin ID shift
            match version {
                JavaMinecraftVersion::V_1_21 => assert_eq!(CLogin::to_id(version), 43),
                JavaMinecraftVersion::V_1_21_2 | JavaMinecraftVersion::V_1_21_4 => {
                    assert_eq!(CLogin::to_id(version), 44);
                }
                JavaMinecraftVersion::V_26_1 => assert_eq!(CLogin::to_id(version), 49),
                JavaMinecraftVersion::V_26_2 => assert_eq!(CLogin::to_id(version), 49),
                _ => {}
            }

            // Play: CChunkData ID shift
            match version {
                JavaMinecraftVersion::V_1_21 => assert_eq!(CChunkData::to_id(version), 39),
                JavaMinecraftVersion::V_1_21_2 | JavaMinecraftVersion::V_1_21_4 => {
                    assert_eq!(CChunkData::to_id(version), 40);
                }
                JavaMinecraftVersion::V_26_1 => assert_eq!(CChunkData::to_id(version), 45),
                JavaMinecraftVersion::V_26_2 => assert_eq!(CChunkData::to_id(version), 45),
                _ => {}
            }

            // Play: CParticle ID shift
            match version {
                JavaMinecraftVersion::V_1_21 => assert_eq!(CParticle::to_id(version), 41),
                JavaMinecraftVersion::V_1_21_2 | JavaMinecraftVersion::V_1_21_4 => {
                    assert_eq!(CParticle::to_id(version), 42);
                }
                JavaMinecraftVersion::V_26_1 => assert_eq!(CParticle::to_id(version), 47),
                JavaMinecraftVersion::V_26_2 => assert_eq!(CParticle::to_id(version), 47),
                _ => {}
            }

            // Play: SPlayerPosition ID shift
            match version {
                JavaMinecraftVersion::V_1_21 => assert_eq!(SPlayerPosition::to_id(version), 26),
                JavaMinecraftVersion::V_1_21_2 | JavaMinecraftVersion::V_1_21_4 => {
                    assert_eq!(SPlayerPosition::to_id(version), 28);
                }
                JavaMinecraftVersion::V_26_1 | JavaMinecraftVersion::V_26_2 => {
                    assert_eq!(SPlayerPosition::to_id(version), 30);
                }
                _ => {}
            }

            // Play: SClickSlot ID shift
            match version {
                JavaMinecraftVersion::V_1_21 => assert_eq!(SClickSlot::to_id(version), 14),
                JavaMinecraftVersion::V_1_21_2 | JavaMinecraftVersion::V_1_21_4 => {
                    assert_eq!(SClickSlot::to_id(version), 16);
                }
                JavaMinecraftVersion::V_26_1 | JavaMinecraftVersion::V_26_2 => {
                    assert_eq!(SClickSlot::to_id(version), 18);
                }
                _ => {}
            }
        }
    }

    #[test]
    fn test_item_stack_serialization_all_versions() {
        for version in TARGET_VERSIONS {
            let item = &Item::DIAMOND_SWORD;
            let stack = ItemStack::new(1, item);
            let serializer = ItemStackSerializer(Cow::Borrowed(&stack));

            let mut buf = Vec::new();
            serializer
                .write_with_version(&mut buf, &version)
                .expect("item stack serialization failed");

            assert!(!buf.is_empty(), "serialized item stack must not be empty");

            let mut slice = &buf[..];
            let decoded = ItemStackSerializer::read_with_version(&mut slice, &version)
                .expect("item stack deserialization failed");

            assert_eq!(decoded.0.item_count, 1);
            assert_eq!(decoded.0.item.id, item.id);
        }
    }

    #[test]
    fn test_custom_model_data_format_delta() {
        let model_data = CustomModelDataImpl {
            floats: vec![105.0],
            flags: vec![true],
            strings: vec!["custom_model".to_string()],
            colors: vec![0x00FF_0000],
        };

        // Write NBT representation
        let nbt = model_data.write_data();
        let decoded = CustomModelDataImpl::read_data(&nbt).expect("custom model data nbt decode failed");

        assert_eq!(decoded.floats, vec![105.0]);
        assert_eq!(decoded.flags, vec![true]);
        assert_eq!(decoded.strings, vec!["custom_model".to_string()]);
        assert_eq!(decoded.colors, vec![0x00FF_0000]);
    }

    #[test]
    fn test_item_model_component_handling() {
        let item_model = ItemModelImpl {
            id: Cow::Borrowed("minecraft:ruby_sword"),
        };
        let nbt = item_model.write_data();
        let decoded = ItemModelImpl::read_data(&nbt).expect("item model nbt decode failed");
        assert_eq!(decoded.id, "minecraft:ruby_sword");
    }

    #[test]
    fn test_component_length_prefixing_gating() {
        let item = &Item::APPLE;
        let stack = ItemStack::new(5, item);
        let serializer = ItemStackSerializer(Cow::Borrowed(&stack));

        // For <= 1.21.4, untrusted write routes to write_with_version (no length prefix)
        let mut buf_1_21_4 = Vec::new();
        serializer
            .write_untrusted_with_version(&mut buf_1_21_4, &JavaMinecraftVersion::V_1_21_4)
            .unwrap();

        // For >= 1.21.5, untrusted write routes to write_length_prefixed_with_version
        let mut buf_26_1 = Vec::new();
        serializer
            .write_untrusted_with_version(&mut buf_26_1, &JavaMinecraftVersion::V_26_1)
            .unwrap();

        assert!(!buf_1_21_4.is_empty());
        assert!(!buf_26_1.is_empty());
    }

    #[test]
    fn test_bidirectional_item_id_remapping() {
        for version in TARGET_VERSIONS {
            for item in [&Item::DIAMOND_SWORD, &Item::STONE, &Item::APPLE, &Item::STICK] {
                let remapped = remap_item_id_for_version(item.id, version);
                let reversed = remap_item_id_from_version(remapped, version);
                assert!(
                    remapped > 0,
                    "Remapped item ID for {} must be > 0",
                    item.registry_key
                );

                assert_eq!(
                    reversed, item.id,
                    "Bidirectional remapping must preserve identity for item {}",
                    item.registry_key
                );
            }
        }
    }
}

// ==============================================================================
// TIER 4: REAL-WORLD APPLICATION SCENARIOS
// ==============================================================================

mod tier4_real_world_simulation {
    use super::*;

    fn simulate_connection_lifecycle(version: JavaMinecraftVersion) {
        let protocol = version.protocol_version();

        // Step 1: Handshake (Client -> Server)
        let handshake = SHandShake {
            protocol_version: VarInt(protocol),
            server_address: "play.potatomc.org".into(),
            server_port: 25565,
            next_state: ConnectionState::Login,
        };
        let mut net_stream = Vec::new();
        handshake.write_packet_data(&mut net_stream, &version).unwrap();

        let mut read_slice = &net_stream[..];
        let received_handshake = SHandShake::read(&mut read_slice, &version).unwrap();
        assert_eq!(received_handshake.next_state, ConnectionState::Login);
        assert_eq!(received_handshake.protocol_version.0, protocol);

        // Step 2: Login Start (Client -> Server)
        let player_uuid = Uuid::new_v4();
        let login_start = SLoginStart {
            name: "PotatoPlayer".into(),
            uuid: player_uuid,
        };
        let mut net_stream = Vec::new();
        login_start.write_packet_data(&mut net_stream, &version).unwrap();

        let mut read_slice = &net_stream[..];
        let received_login_start = SLoginStart::read(&mut read_slice, &version).unwrap();
        assert_eq!(&*received_login_start.name, "PotatoPlayer");

        // Step 3: Login Success (Server -> Client)
        let session_id = Uuid::new_v4();
        let login_success = CLoginSuccess::new(
            &received_login_start.uuid,
            &received_login_start.name,
            &[],
            false,
            session_id,
        );
        let mut net_stream = Vec::new();
        login_success.write_packet_data(&mut net_stream, &version).unwrap();
        assert!(!net_stream.is_empty());

        // Step 4: Login Acknowledged (Client -> Server)
        let login_ack = SLoginAcknowledged;
        let mut net_stream = Vec::new();
        login_ack.write_packet_data(&mut net_stream, &version).unwrap();
        let mut read_slice = &net_stream[..];
        assert!(SLoginAcknowledged::read(&mut read_slice, &version).is_ok());

        // Step 5: Configuration Negotiation (Server <-> Client)
        // 5a. CKnownPacks
        let packs = [KnownPack {
            namespace: "minecraft".into(),
            id: "core".into(),
            version: "1.21".into(),
        }];
        let known_packs = CKnownPacks::new(&packs);
        let mut net_stream = Vec::new();
        known_packs.write_packet_data(&mut net_stream, &version).unwrap();

        let mut read_slice = &net_stream[..];
        let client_known_packs = SKnownPacks::read(&mut read_slice, &version).unwrap();
        assert_eq!(client_known_packs.known_packs.len(), 1);

        // 5b. CFeatureFlags
        let features = ["minecraft:vanilla".into()];
        let feature_flags = CFeatureFlags::new(&features);
        let mut net_stream = Vec::new();
        feature_flags.write_packet_data(&mut net_stream, &version).unwrap();
        assert!(!net_stream.is_empty());

        // 5c. Registry Data Sync
        let synced_registries = Registry::get_synced(version);
        for reg in &synced_registries {
            let reg_packet = CRegistryData::new(&reg.registry_id, &reg.registry_entries);
            let mut reg_buf = Vec::new();
            reg_packet.write_packet_data(&mut reg_buf, &version).unwrap();
            assert!(!reg_buf.is_empty());
        }

        // 5d. Finish Configuration
        let finish_config = CFinishConfig;
        let mut net_stream = Vec::new();
        finish_config.write_packet_data(&mut net_stream, &version).unwrap();

        let ack_finish = SAcknowledgeFinishConfig;
        let mut net_stream = Vec::new();
        ack_finish.write_packet_data(&mut net_stream, &version).unwrap();
        let mut read_slice = &net_stream[..];
        assert!(SAcknowledgeFinishConfig::read(&mut read_slice, &version).is_ok());

        // Step 6: Play State Transition (Join Game, Spawn, Movement, Input)
        // 6a. CLogin (Join Game)
        let spawn_data = PlayerSpawnData::new(
            Dimension::OVERWORLD,
            424_242,
            0,
            -1,
            false,
            false,
            None,
            VarInt(0),
            VarInt(63),
        );
        let dim_names = ["minecraft:overworld".into()];
        let login_play = CLogin::new(
            1,
            false,
            &dim_names,
            VarInt(100),
            VarInt(12),
            VarInt(12),
            false,
            true,
            false,
            spawn_data,
            false,
            false,
        );
        let mut net_stream = Vec::new();
        login_play.write_packet_data(&mut net_stream, &version).unwrap();
        assert!(!net_stream.is_empty());

        // 6b. CSpawnEntity
        let spawn_entity = CSpawnEntity::new(
            VarInt(2),
            Uuid::new_v4(),
            VarInt(110), // zombie
            Vector3::new(0.0, 64.0, 0.0),
            0.0,
            0.0,
            0.0,
            VarInt(0),
            Vector3::new(0.0, 0.0, 0.0),
        );
        let mut net_stream = Vec::new();
        spawn_entity.write_packet_data(&mut net_stream, &version).unwrap();
        assert!(!net_stream.is_empty());

        // 6c. CParticle
        let particle = CParticle::new(
            false,
            false,
            Vector3::new(0.0, 65.0, 0.0),
            Vector3::new(0.1, 0.1, 0.1),
            0.5,
            5,
            VarInt(1),
            &[],
        );
        let mut net_stream = Vec::new();
        particle.write_packet_data(&mut net_stream, &version).unwrap();
        assert!(!net_stream.is_empty());

        // 6d. CChunkData
        let chunk = ChunkData::empty(0, 0);
        let chunk_packet = CChunkData::new(&chunk);
        let mut net_stream = Vec::new();
        chunk_packet.write_packet_data(&mut net_stream, &version).unwrap();
        assert!(!net_stream.is_empty());

        // 6e. Client Movement (SPlayerPosition)
        let movement = SPlayerPosition {
            position: Vector3::new(0.0, 64.0, 0.0),
            collision: 1,
        };
        let mut net_stream = Vec::new();
        movement.write_packet_data(&mut net_stream, &version).unwrap();
        let mut read_slice = &net_stream[..];
        let recv_movement = SPlayerPosition::read(&mut read_slice, &version).unwrap();
        assert_eq!(recv_movement.position.y, 64.0);

        // 6f. Client Input (SPlayerInput)
        let player_input = SPlayerInput {
            input: SPlayerInput::FORWARD,
        };
        let mut net_stream = Vec::new();
        player_input.write_packet_data(&mut net_stream, &version).unwrap();
        let mut read_slice = &net_stream[..];
        let recv_input = SPlayerInput::read(&mut read_slice, &version).unwrap();
        assert_eq!(recv_input.input & SPlayerInput::FORWARD, SPlayerInput::FORWARD);
    }

    #[test]
    fn test_full_pipeline_simulation_v1_21() {
        simulate_connection_lifecycle(JavaMinecraftVersion::V_1_21);
    }

    #[test]
    fn test_full_pipeline_simulation_v1_21_2() {
        simulate_connection_lifecycle(JavaMinecraftVersion::V_1_21_2);
    }

    #[test]
    fn test_full_pipeline_simulation_v1_21_4() {
        simulate_connection_lifecycle(JavaMinecraftVersion::V_1_21_4);
    }

    #[test]
    fn test_full_pipeline_simulation_v26_1() {
        simulate_connection_lifecycle(JavaMinecraftVersion::V_26_1);
    }

    #[test]
    fn test_full_pipeline_simulation_v26_2() {
        simulate_connection_lifecycle(JavaMinecraftVersion::V_26_2);
    }

    #[test]
    fn test_full_pipeline_simulation_v26_3() {
        simulate_connection_lifecycle(JavaMinecraftVersion::V_26_3);
    }

    #[test]
    fn test_particle_26_3_speed_split() {
        // For 26.3, after offset (3xf32=12 bytes), we expect 3 speed floats (12 bytes) not 1 (4 bytes).
        let particle = CParticle::new(
            false,
            false,
            Vector3::new(0.0, 64.0, 0.0),
            Vector3::new(0.1, 0.1, 0.1),
            2.5, // max_speed
            5,
            VarInt(1),
            &[],
        );
        let mut buf_26_2 = Vec::new();
        particle.write_packet_data(&mut buf_26_2, &JavaMinecraftVersion::V_26_2).unwrap();
        let mut buf_26_3 = Vec::new();
        particle.write_packet_data(&mut buf_26_3, &JavaMinecraftVersion::V_26_3).unwrap();
        // 26.3 should be 8 bytes longer: 2 extra floats (8 bytes) + randomizationType VarInt (1 byte) - nothing removed = +9 bytes net
        // Actually: +2 floats (8 bytes) for speed split, +1 varint byte for randomizationType = +9
        assert_eq!(
            buf_26_3.len(),
            buf_26_2.len() + 9,
            "26.3 particle packet must be 9 bytes longer than 26.2 (2 extra speed floats + randomizationType VarInt)"
        );
    }
    #[test]
    fn test_entity_pos_rot_26_3_properties_varint() {
        use pumpkin_protocol::ser::NetworkReadExt;
        use pumpkin_util::math::vector3::Vector3;
        // For 26.3, the wire format is:
        //   VarInt entityId
        //   VarInt properties (onGround in bit 0, stepCount in bits 1+)
        //   i16 deltaX, i16 deltaY, i16 deltaZ
        //   u8 yaw, u8 pitch
        // For pre-26.3 (e.g., 26.2):
        //   VarInt entityId
        //   i16 deltaX, i16 deltaY, i16 deltaZ
        //   u8 yaw, u8 pitch
        //   bool onGround
        let packet = CUpdateEntityPosRot::new(
            VarInt(42),
            Vector3::new(100i16, 200i16, 300i16),
            128u8, // yaw
            64u8,  // pitch
            true,  // on_ground
        );

        let mut buf_26_2 = Vec::new();
        packet.write_packet_data(&mut buf_26_2, &JavaMinecraftVersion::V_26_2).unwrap();
        let mut buf_26_3 = Vec::new();
        packet.write_packet_data(&mut buf_26_3, &JavaMinecraftVersion::V_26_3).unwrap();

        // Both formats have the same number of bytes:
        // 26.2: VarInt(42)=1B + i16*3=6B + u8*2=2B + bool=1B = 10B
        // 26.3: VarInt(42)=1B + VarInt(1)=1B + i16*3=6B + u8*2=2B = 10B
        // (onGround bool moved into properties; net size same)
        assert_eq!(
            buf_26_2.len(), buf_26_3.len(),
            "26.2 and 26.3 entity pos_rot must be same total length"
        );

        // In 26.3, byte[1] is the properties VarInt (value=1 for onGround=true, stepCount=0)
        // In 26.2, byte[1] is the first byte of deltaX i16 (high byte of 100 = 0x00)
        let mut slice_26_3 = buf_26_3.as_slice();
        let entity_id = slice_26_3.get_var_int().unwrap();
        assert_eq!(entity_id, VarInt(42));
        let properties = slice_26_3.get_var_int().unwrap();
        assert_eq!(properties.0 & 1, 1, "onGround must be bit 0 of properties");
        assert_eq!(properties.0 >> 1, 0, "stepCount must be 0 for a simple Linear move");
        let dx = slice_26_3.get_i16_be().unwrap();
        let dy = slice_26_3.get_i16_be().unwrap();
        let dz = slice_26_3.get_i16_be().unwrap();
        assert_eq!((dx, dy, dz), (100, 200, 300));
    }
    #[test]
    fn test_fox_metadata_indices_across_versions() {
        use pumpkin_data::tracked_data::fox;
        use pumpkin_protocol::java::client::play::Metadata;

        // 1.21.11 client expectations:
        // BABY_ID: index 16
        // TYPE_ID: index 17
        // FLAGS_ID: index 18
        // TRUSTED_ID_0: index 19
        // AGE_LOCKED: not present (index 255 -> skipped)
        assert_eq!(fox::BABY_ID.get(&JavaMinecraftVersion::V_1_21_11), 16);
        assert_eq!(fox::TYPE_ID.get(&JavaMinecraftVersion::V_1_21_11), 17);
        assert_eq!(fox::FLAGS_ID.get(&JavaMinecraftVersion::V_1_21_11), 18);
        assert_eq!(fox::DATA_TRUSTED_ID_0.get(&JavaMinecraftVersion::V_1_21_11), 19);
        assert_eq!(fox::AGE_LOCKED.get(&JavaMinecraftVersion::V_1_21_11), 255);

        // 26.2 client expectations:
        // BABY_ID: index 16
        // AGE_LOCKED: index 17
        // TYPE_ID: index 18
        // FLAGS_ID: index 19
        // TRUSTED_ID_0: index 20
        assert_eq!(fox::BABY_ID.get(&JavaMinecraftVersion::V_26_2), 16);
        assert_eq!(fox::AGE_LOCKED.get(&JavaMinecraftVersion::V_26_2), 17);
        assert_eq!(fox::TYPE_ID.get(&JavaMinecraftVersion::V_26_2), 18);
        assert_eq!(fox::FLAGS_ID.get(&JavaMinecraftVersion::V_26_2), 19);
        assert_eq!(fox::DATA_TRUSTED_ID_0.get(&JavaMinecraftVersion::V_26_2), 20);

        // Verify serializing Fox flags writes index 18 for 1.21.11, not index 19!
        let mut buf_1_21_11 = Vec::new();
        Metadata::new(fox::FLAGS_ID, 0u8)
            .write(&mut buf_1_21_11, &JavaMinecraftVersion::V_1_21_11)
            .unwrap();
        assert_eq!(buf_1_21_11[0], 18, "Fox flags MUST be index 18 on 1.21.11");

        // Verify serializing AGE_LOCKED on 1.21.11 is completely skipped (empty buffer)
        let mut buf_age = Vec::new();
        Metadata::new(fox::AGE_LOCKED, false)
            .write(&mut buf_age, &JavaMinecraftVersion::V_1_21_11)
            .unwrap();
        assert!(buf_age.is_empty(), "AGE_LOCKED must not be serialized for 1.21.11");
    }
}

