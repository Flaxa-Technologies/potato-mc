use pumpkin_data::translation;
use pumpkin_protocol::{ConnectionState, java::server::handshake::SHandShake};
use pumpkin_util::{text::TextComponent, version::JavaMinecraftVersion};
use tracing::{debug, info};

use crate::net::java::pending::PendingConnection;

impl PendingConnection {
    pub async fn handle_handshake(&mut self, handshake: SHandShake) {
        let version = handshake.protocol_version.0 as u32;
        self.server_address = handshake.server_address.to_string();
        self.version
            .store(JavaMinecraftVersion::from_protocol(version));

        debug!("Handshake: next state is {:?}", &handshake.next_state);
        self.connection_state.store(handshake.next_state);
        if self.connection_state.load() != ConnectionState::Status {
            let protocol = handshake.protocol_version.0;
            let min_protocol = JavaMinecraftVersion::V_1_21.protocol_version();
            let max_protocol = JavaMinecraftVersion::V_26_3.protocol_version();
            let version_range = "1.21 - 26.3";
            info!(
                "Client [{}] connected with protocol {} ({})",
                self.address,
                protocol,
                self.version.load()
            );
            if protocol < min_protocol {
                self.kick(TextComponent::translate_cross(
                    translation::java::MULTIPLAYER_DISCONNECT_OUTDATED_CLIENT,
                    translation::bedrock::DISCONNECTIONSCREEN_OUTDATEDCLIENT,
                    [TextComponent::text(version_range)],
                ))
                .await;
            } else if protocol > max_protocol {
                self.kick(TextComponent::translate_cross(
                    translation::java::MULTIPLAYER_DISCONNECT_OUTDATED_SERVER,
                    translation::bedrock::DISCONNECTIONSCREEN_OUTDATEDSERVER,
                    [TextComponent::text(version_range)],
                ))
                .await;
            }
        }
    }
}
