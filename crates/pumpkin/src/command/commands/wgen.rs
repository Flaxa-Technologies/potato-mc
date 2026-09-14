use std::sync::Arc;

use crate::command::argument_builder::{ArgumentBuilder, argument, command};
use crate::command::argument_types::core::string::StringArgumentType;
use crate::command::context::command_context::CommandContext;
use crate::command::node::dispatcher::CommandDispatcher;
use crate::command::node::{CommandExecutor, CommandExecutorResult};
use pumpkin_util::PermissionLvl;
use pumpkin_util::permission::{Permission, PermissionDefault, PermissionRegistry};
use pumpkin_util::text::TextComponent;
use pumpkin_util::text::color::NamedColor;
use pumpkin_util::world_seed::Seed;
use pumpkin_world::generation::get_world_gen;

const DESCRIPTION: &str = "Regenerates a world with a given seed.";
const PERMISSION: &str = "minecraft:command.wgen";

const ARG_WORLD: &str = "world";
const ARG_SEED: &str = "seed";

struct WGenExecutor;

impl CommandExecutor for WGenExecutor {
    fn execute(&self, context: &CommandContext) -> CommandExecutorResult {
        let world_name = StringArgumentType::get(context, ARG_WORLD)?;
        let seed_str = StringArgumentType::get(context, ARG_SEED)?;

        let seed_val: i64 = match seed_str.parse::<i64>() {
            Ok(s) => s,
            Err(_) => {
                // Java-style string hash code fallback
                let mut h: i64 = 0;
                for b in seed_str.bytes() {
                    h = h.wrapping_mul(31).wrapping_add(b as i64);
                }
                h
            }
        };

        let worlds = context.server().worlds.load();
        let target_world = worlds.iter().find(|w| {
            let full_name = w.dimension.minecraft_name.to_lowercase();
            let short_name = full_name.split(':').last().unwrap_or(&full_name);
            let target = world_name.to_lowercase();

            full_name == target
                || short_name == target
                || (target == "nether" && (full_name == "minecraft:the_nether" || short_name == "the_nether"))
                || (target == "overworld" && short_name == "overworld")
                || (target == "end" && (full_name == "minecraft:the_end" || short_name == "the_end"))
        });

        let Some(world) = target_world else {
            context.source.send_feedback(
                TextComponent::text(format!("Unknown world '{world_name}'. Available: overworld, nether, the_end"))
                    .color_named(NamedColor::Red),
                false,
            );
            return Ok(0);
        };

        let dim_name = world.dimension.minecraft_name.to_string();

        // 1. Clear all in-memory loaded chunks and scheduled tick queues
        world.level.loaded_chunks.clear();
        world.level.chunks_with_scheduled_ticks.clear();

        // 2. Re-create the world generator with the new seed
        let new_world_gen = get_world_gen(
            Seed(seed_val as u64),
            world.dimension.clone(),
            false,
            Vec::new(),
            String::new(),
        );
        world.level.world_gen.store(Arc::new(*new_world_gen));

        tracing::info!(
            "[WGEN] World '{}' regenerated with seed {} (raw '{}')",
            dim_name,
            seed_val,
            seed_str
        );

        context.source.send_feedback(
            TextComponent::text(format!(
                "Successfully regenerated world '{dim_name}' with seed {seed_val}"
            ))
            .color_named(NamedColor::Green),
            true,
        );

        Ok(1)
    }
}

pub fn register(dispatcher: &mut CommandDispatcher, registry: &PermissionRegistry) {
    registry.register_permission_or_panic(Permission::new(
        PERMISSION,
        DESCRIPTION,
        PermissionDefault::Op(PermissionLvl::Two),
    ));

    dispatcher.register(
        command("wgen", DESCRIPTION)
            .requires(PERMISSION)
            .then(
                argument(ARG_WORLD, StringArgumentType::SingleWord).then(
                    argument(ARG_SEED, StringArgumentType::SingleWord).executes(WGenExecutor),
                ),
            ),
    );
}
