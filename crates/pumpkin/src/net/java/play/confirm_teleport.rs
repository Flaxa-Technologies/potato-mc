#[allow(clippy::wildcard_imports)]
use super::*;

impl JavaClient {
    pub fn handle_confirm_teleport(&self, player: &Player, confirm_teleport: &SConfirmTeleport) {
        enum TeleportResult {
            Success,
            StaleId,
            WrongId,
            NotTeleporting,
        }

        let (result, expected_id, _position_confirmed) = {
            let mut awaiting_teleport = player
                .awaiting_teleport
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Some((id, position)) = awaiting_teleport.as_ref() {
                let exp_id = *id;
                let pos = *position;
                if &exp_id == &confirm_teleport.teleport_id {
                    player.get_entity().set_pos(pos);
                    *awaiting_teleport = None;
                    (TeleportResult::Success, Some(exp_id), Some(pos))
                } else if confirm_teleport.teleport_id.0 < exp_id.0 {
                    // Stale confirmation for an earlier teleport packet that was superseded by a newer one.
                    // Vanilla Minecraft (ServerGamePacketListenerImpl.java:537) simply ignores stale IDs.
                    (TeleportResult::StaleId, Some(exp_id), Some(pos))
                } else {
                    (TeleportResult::WrongId, Some(exp_id), Some(pos))
                }
            } else {
                (TeleportResult::NotTeleporting, None, None)
            }
        };

        tracing::info!(
            "[TELEPORT-CONFIRM] received_id={} expected_id={:?} action={}",
            confirm_teleport.teleport_id.0,
            expected_id.map(|i| i.0),
            match result {
                TeleportResult::Success => "accepted",
                TeleportResult::StaleId => "stale",
                TeleportResult::WrongId | TeleportResult::NotTeleporting => "rejected",
            }
        );

        match result {
            TeleportResult::Success | TeleportResult::StaleId | TeleportResult::NotTeleporting => {}
            TeleportResult::WrongId => {
                self.try_kick(&TextComponent::text("Wrong teleport id"));
            }
        }
    }
}
