#[allow(clippy::wildcard_imports)]
use super::*;
use pumpkin_protocol::java::client::play::CommandSuggestion;

impl JavaClient {
    pub fn handle_command_suggestion(
        &self,
        player: &Arc<Player>,
        packet: &SCommandSuggestion<'_>,
        server: &Arc<Server>,
    ) {
        let has_slash = packet.command.starts_with('/');
        let cmd = if has_slash {
            &packet.command[1..]
        } else {
            packet.command
        };

        let suggestions = server
            .command_dispatcher
            .load()
            .suggest_with_range(cmd, &player.get_command_source(server));

        let offset = if has_slash { 1 } else { 0 };
        let start = (suggestions.range.start + offset) as i32;
        let length = suggestions.range.len() as i32;

        let matches: Vec<CommandSuggestion> = suggestions
            .suggestions
            .into_iter()
            .map(|s| CommandSuggestion::new(s.text.cached_text().clone(), s.tooltip))
            .collect();

        let response = CCommandSuggestions::new(
            packet.id,
            start.into(),
            length.into(),
            matches.into_boxed_slice(),
        );

        player.try_send_client_packet(&response);
    }
}
