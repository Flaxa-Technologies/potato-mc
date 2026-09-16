use pumpkin_util::PermissionLvl;
use pumpkin_util::permission::{Permission, PermissionDefault, PermissionRegistry};
use pumpkin_util::text::TextComponent;

use crate::command::argument_builder::{ArgumentBuilder, argument, command, literal};
use crate::command::argument_types::core::float::FloatArgumentType;
use crate::command::argument_types::core::integer::IntegerArgumentType;
use crate::command::argument_types::core::string::StringArgumentType;
use crate::command::context::command_context::CommandContext;
use crate::command::node::dispatcher::CommandDispatcher;
use crate::command::node::{CommandExecutor, CommandExecutorResult};

const DESCRIPTION: &str = "Evaluates number providers for loot or datapack execution.";
const PERMISSION: &str = "minecraft:command.compute";

pub fn register(dispatcher: &mut CommandDispatcher, registry: &PermissionRegistry) {
    registry.register_permission_or_panic(Permission::new(
        PERMISSION,
        DESCRIPTION,
        PermissionDefault::Op(PermissionLvl::Two),
    ));

    dispatcher.register(
        command("compute", DESCRIPTION)
            .requires(PERMISSION)
            .then(
                literal("float")
                    .then(
                        argument("provider", FloatArgumentType::any())
                            .executes(ComputeFloatExecutor { has_scale: false })
                            .then(
                                argument("scale", FloatArgumentType::any())
                                    .executes(ComputeFloatExecutor { has_scale: true }),
                            ),
                    )
                    .then(
                        argument("provider_str", StringArgumentType::SingleWord)
                            .executes(ComputeFloatStrExecutor { has_scale: false })
                            .then(
                                argument("scale", FloatArgumentType::any())
                                    .executes(ComputeFloatStrExecutor { has_scale: true }),
                            ),
                    ),
            )
            .then(
                literal("integer")
                    .then(
                        argument("provider", IntegerArgumentType::any())
                            .executes(ComputeIntExecutor),
                    )
                    .then(
                        argument("provider_str", StringArgumentType::SingleWord)
                            .executes(ComputeIntStrExecutor),
                    ),
            ),
    );
}

struct ComputeFloatExecutor {
    has_scale: bool,
}

impl CommandExecutor for ComputeFloatExecutor {
    fn execute(&self, context: &CommandContext) -> CommandExecutorResult {
        let original = FloatArgumentType::get(context, "provider")?;
        let scale = if self.has_scale {
            FloatArgumentType::get(context, "scale")?
        } else {
            1.0
        };

        let scaled = original * scale;
        let result = scaled.floor() as i32;

        if (result as f32) == scaled {
            context.source.send_feedback(
                TextComponent::translate_cross(
                    "command.compute.result.unnamed.exact",
                    "command.compute.result.unnamed.exact",
                    [TextComponent::text(result.to_string())],
                ),
                false,
            );
        } else {
            context.source.send_feedback(
                TextComponent::translate_cross(
                    "command.compute.result.unnamed.rounded",
                    "command.compute.result.unnamed.rounded",
                    [
                        TextComponent::text(scaled.to_string()),
                        TextComponent::text(result.to_string()),
                    ],
                ),
                false,
            );
        }

        Ok(result)
    }
}

struct ComputeFloatStrExecutor {
    has_scale: bool,
}

impl CommandExecutor for ComputeFloatStrExecutor {
    fn execute(&self, context: &CommandContext) -> CommandExecutorResult {
        let str_val = StringArgumentType::get(context, "provider_str")?;
        let original = str_val.parse::<f32>().unwrap_or(0.0);
        let scale = if self.has_scale {
            FloatArgumentType::get(context, "scale")?
        } else {
            1.0
        };

        let scaled = original * scale;
        let result = scaled.floor() as i32;

        if (result as f32) == scaled {
            context.source.send_feedback(
                TextComponent::translate_cross(
                    "command.compute.result.named.exact",
                    "command.compute.result.named.exact",
                    [
                        TextComponent::text(str_val.to_string()),
                        TextComponent::text(result.to_string()),
                    ],
                ),
                false,
            );
        } else {
            context.source.send_feedback(
                TextComponent::translate_cross(
                    "command.compute.result.named.rounded",
                    "command.compute.result.named.rounded",
                    [
                        TextComponent::text(str_val.to_string()),
                        TextComponent::text(scaled.to_string()),
                        TextComponent::text(result.to_string()),
                    ],
                ),
                false,
            );
        }

        Ok(result)
    }
}

struct ComputeIntExecutor;

impl CommandExecutor for ComputeIntExecutor {
    fn execute(&self, context: &CommandContext) -> CommandExecutorResult {
        let result = IntegerArgumentType::get(context, "provider")?;

        context.source.send_feedback(
            TextComponent::translate_cross(
                "command.compute.result.unnamed.exact",
                "command.compute.result.unnamed.exact",
                [TextComponent::text(result.to_string())],
            ),
            false,
        );

        Ok(result)
    }
}

struct ComputeIntStrExecutor;

impl CommandExecutor for ComputeIntStrExecutor {
    fn execute(&self, context: &CommandContext) -> CommandExecutorResult {
        let str_val = StringArgumentType::get(context, "provider_str")?;
        let result = str_val.parse::<i32>().unwrap_or(0);

        context.source.send_feedback(
            TextComponent::translate_cross(
                "command.compute.result.named.exact",
                "command.compute.result.named.exact",
                [
                    TextComponent::text(str_val.to_string()),
                    TextComponent::text(result.to_string()),
                ],
            ),
            false,
        );

        Ok(result)
    }
}
