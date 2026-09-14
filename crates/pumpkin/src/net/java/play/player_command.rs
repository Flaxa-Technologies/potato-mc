#[allow(clippy::wildcard_imports)]
use super::*;

impl JavaClient {
    pub fn handle_player_command(
        &self,
        player: &Arc<Player>,
        command: &SPlayerCommand,
        server: &Arc<Server>,
    ) {
        if command.entity_id != player.entity_id().into() {
            return;
        }
        if !player.has_client_loaded() {
            return;
        }
        player.update_last_action_time();

        let entity = &player.get_entity();
        match command.action {
            Action::StartSprinting => {
                if !entity.is_sprinting() {
                    send_cancellable_blocking! {{
                        server;
                        PlayerToggleSprintEvent::new(player.clone(), true);
                        'after: {
                            player.set_sprinting(event.is_sprinting);
                            player.update_player_pose();
                        }
                    }}
                }
            }
            Action::StopSprinting => {
                if entity.is_sprinting() {
                    send_cancellable_blocking! {{
                        server;
                        PlayerToggleSprintEvent::new(player.clone(), false);
                        'after: {
                            player.set_sprinting(event.is_sprinting);
                            player.update_player_pose();
                        }
                    }}
                }
            }
            Action::LeaveBed => player.wake_up(),

            Action::StartHorseJump | Action::StopHorseJump => {
                debug!("todo");
            }
            Action::OpenVehicleInventory => {
                if let Some(vehicle) = entity.get_vehicle() {
                    vehicle.open_custom_inventory_screen(player);
                }
            }
            Action::StartFlyingElytra => {
                let can_glide = player.can_glide();
                if can_glide {
                    if !entity.is_fall_flying() {
                        let mut event = crate::plugin::api::events::entity::entity_toggle_glide::EntityToggleGlideEvent::new(
                            entity.entity_id,
                            true,
                        );
                        server.plugin_manager.fire_blocking(server, &mut event);
                        if !event.cancelled {
                            entity.set_fall_flying(event.is_gliding);
                        }
                    }
                } else if entity.is_fall_flying() {
                    entity.set_fall_flying(false);
                }
            }
            // <= 1.21.5
            Action::StartSneaking | Action::StopSneaking => {
                self.handle_player_input(
                    player,
                    &SPlayerInput {
                        input: SPlayerInput::SNEAK,
                    },
                    server,
                );
            }
        }
    }
}
