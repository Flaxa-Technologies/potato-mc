use std::collections::HashMap;
use std::sync::Arc;

use crate::host::HostConsole;
use crate::player::Player;

/// Errors that can occur during command parsing or execution, matching Paper's error model.
#[derive(Debug, Clone, PartialEq)]
pub enum CommandError {
    /// Command can only be executed by a player, not the console.
    RequiresPlayer,
    /// The sender lacks the required permission node.
    NoPermission(String),
    /// A required argument was missing from the command input.
    MissingArgument(String),
    /// An argument failed validation or type parsing.
    InvalidArgument {
        argument: String,
        expected: String,
        found: String,
    },
    /// An unknown subcommand was specified.
    UnknownSubcommand(String),
    /// Command usage syntax error.
    Usage(String),
    /// Generic or user-defined execution failure.
    ExecutionFailed(String),
}

impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RequiresPlayer => write!(f, "This command can only be executed by in-game players."),
            Self::NoPermission(perm) => write!(f, "I'm sorry, but you do not have permission to perform this command ({perm})."),
            Self::MissingArgument(arg) => write!(f, "Missing required argument: <{arg}>"),
            Self::InvalidArgument { argument, expected, found } => {
                write!(f, "Invalid argument for '{argument}': expected {expected}, found '{found}'")
            }
            Self::UnknownSubcommand(sub) => write!(f, "Unknown subcommand: '{sub}'"),
            Self::Usage(usage) => write!(f, "Usage: {usage}"),
            Self::ExecutionFailed(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for CommandError {}

/// Represents the source of a command execution (Console or In-Game Player).
#[derive(Clone)]
pub enum CommandSender {
    Console(Arc<dyn HostConsole>),
    Player(Player),
}

impl CommandSender {
    /// Sends a standard chat/output message to the sender.
    pub fn send_message(&self, message: &str) {
        match self {
            Self::Console(c) => c.send_message(message),
            Self::Player(p) => p.send_message(message),
        }
    }

    /// Sends an error message to the sender.
    pub fn send_error(&self, message: &str) {
        self.send_message(&format!("§c{message}"));
    }

    /// Returns a reference to the player if this command was executed by an in-game player.
    pub fn as_player(&self) -> Option<&Player> {
        match self {
            Self::Player(p) => Some(p),
            Self::Console(_) => None,
        }
    }

    /// Requires the sender to be an in-game player, returning `CommandError::RequiresPlayer` otherwise.
    pub fn require_player(&self) -> Result<&Player, CommandError> {
        self.as_player().ok_or(CommandError::RequiresPlayer)
    }

    /// Checks if the sender is an in-game player.
    pub fn is_player(&self) -> bool {
        self.as_player().is_some()
    }

    /// Checks if the sender is the server console.
    pub fn is_console(&self) -> bool {
        matches!(self, Self::Console(_))
    }

    /// Checks whether the sender has the specified permission.
    pub fn has_permission(&self, permission: &str) -> bool {
        match self {
            Self::Console(_) => true,
            Self::Player(p) => p.has_permission(permission),
        }
    }

    /// Returns a display name for this sender.
    pub fn name(&self) -> String {
        match self {
            Self::Console(_) => "Console".to_string(),
            Self::Player(p) => p.name(),
        }
    }
}

/// Supported argument types for structured parsing and client-side tab completion.
#[derive(Debug, Clone, PartialEq)]
pub enum ArgumentType {
    /// A single word (no spaces).
    Word,
    /// A string that may be quoted if containing spaces.
    String,
    /// Greedy string capturing the remainder of the command line.
    GreedyString,
    /// An integer with optional min/max bounds.
    Integer { min: Option<i32>, max: Option<i32> },
    /// A floating-point number with optional min/max bounds.
    Float { min: Option<f32>, max: Option<f32> },
    /// A boolean ("true" or "false").
    Boolean,
    /// An online player target by username.
    Player,
}

/// Parsed typed value stored in the command context.
#[derive(Debug, Clone, PartialEq)]
pub enum ParsedValue {
    String(String),
    Integer(i32),
    Float(f32),
    Boolean(bool),
    Player(String),
}

pub type SuggestionProvider = Arc<dyn Fn(&CommandContext) -> Vec<String> + Send + Sync>;

/// Represents a structured command argument with optional validation and tab-completion suggestions.
#[derive(Clone)]
pub struct Argument {
    pub name: String,
    pub arg_type: ArgumentType,
    pub suggestions: Option<SuggestionProvider>,
}

impl Argument {
    pub fn word(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            arg_type: ArgumentType::Word,
            suggestions: None,
        }
    }

    pub fn string(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            arg_type: ArgumentType::String,
            suggestions: None,
        }
    }

    pub fn greedy_string(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            arg_type: ArgumentType::GreedyString,
            suggestions: None,
        }
    }

    pub fn integer(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            arg_type: ArgumentType::Integer { min: None, max: None },
            suggestions: None,
        }
    }

    pub fn integer_range(name: impl Into<String>, min: i32, max: i32) -> Self {
        Self {
            name: name.into(),
            arg_type: ArgumentType::Integer { min: Some(min), max: Some(max) },
            suggestions: None,
        }
    }

    pub fn float(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            arg_type: ArgumentType::Float { min: None, max: None },
            suggestions: None,
        }
    }

    pub fn float_range(name: impl Into<String>, min: f32, max: f32) -> Self {
        Self {
            name: name.into(),
            arg_type: ArgumentType::Float { min: Some(min), max: Some(max) },
            suggestions: None,
        }
    }

    pub fn boolean(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            arg_type: ArgumentType::Boolean,
            suggestions: None,
        }
    }

    pub fn player(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            arg_type: ArgumentType::Player,
            suggestions: None,
        }
    }

    /// Attaches a custom dynamic suggestion provider for tab completion.
    pub fn suggests<F>(mut self, provider: F) -> Self
    where
        F: Fn(&CommandContext) -> Vec<String> + Send + Sync + 'static,
    {
        self.suggestions = Some(Arc::new(provider));
        self
    }
}

/// The context of a command execution, providing access to sender, arguments, and server lookups.
#[derive(Clone)]
pub struct CommandContext {
    pub(crate) sender: CommandSender,
    pub(crate) label: String,
    pub(crate) parsed_args: HashMap<String, ParsedValue>,
    pub(crate) raw_args: Vec<String>,
    pub(crate) player_resolver: Option<Arc<dyn Fn(&str) -> Option<Player> + Send + Sync>>,
}

impl CommandContext {
    pub fn new(
        sender: CommandSender,
        label: String,
        parsed_args: HashMap<String, ParsedValue>,
        raw_args: Vec<String>,
        player_resolver: Option<Arc<dyn Fn(&str) -> Option<Player> + Send + Sync>>,
    ) -> Self {
        Self {
            sender,
            label,
            parsed_args,
            raw_args,
            player_resolver,
        }
    }

    /// Accesses the command sender.
    pub fn sender(&self) -> &CommandSender {
        &self.sender
    }

    /// Returns the player if the sender is an in-game player, or returns `CommandError::RequiresPlayer`.
    pub fn player(&self) -> Result<&Player, CommandError> {
        self.sender.require_player()
    }

    /// The alias or command label that was used to trigger this command.
    pub fn label(&self) -> &str {
        &self.label
    }

    /// The raw string arguments passed to the command.
    pub fn raw_args(&self) -> &[String] {
        &self.raw_args
    }

    /// Retrieves a parsed string argument by its registered name.
    pub fn get_string(&self, name: &str) -> Result<&str, CommandError> {
        match self.parsed_args.get(name) {
            Some(ParsedValue::String(s)) => Ok(s.as_str()),
            Some(_) => Err(CommandError::InvalidArgument {
                argument: name.to_string(),
                expected: "string".to_string(),
                found: "other type".to_string(),
            }),
            None => Err(CommandError::MissingArgument(name.to_string())),
        }
    }

    /// Retrieves a parsed integer argument by its registered name.
    pub fn get_int(&self, name: &str) -> Result<i32, CommandError> {
        match self.parsed_args.get(name) {
            Some(ParsedValue::Integer(i)) => Ok(*i),
            Some(_) => Err(CommandError::InvalidArgument {
                argument: name.to_string(),
                expected: "integer".to_string(),
                found: "other type".to_string(),
            }),
            None => Err(CommandError::MissingArgument(name.to_string())),
        }
    }

    /// Retrieves a parsed float argument by its registered name.
    pub fn get_float(&self, name: &str) -> Result<f32, CommandError> {
        match self.parsed_args.get(name) {
            Some(ParsedValue::Float(f)) => Ok(*f),
            Some(_) => Err(CommandError::InvalidArgument {
                argument: name.to_string(),
                expected: "float".to_string(),
                found: "other type".to_string(),
            }),
            None => Err(CommandError::MissingArgument(name.to_string())),
        }
    }

    /// Retrieves a parsed boolean argument by its registered name.
    pub fn get_bool(&self, name: &str) -> Result<bool, CommandError> {
        match self.parsed_args.get(name) {
            Some(ParsedValue::Boolean(b)) => Ok(*b),
            Some(_) => Err(CommandError::InvalidArgument {
                argument: name.to_string(),
                expected: "boolean".to_string(),
                found: "other type".to_string(),
            }),
            None => Err(CommandError::MissingArgument(name.to_string())),
        }
    }

    /// Retrieves a target player argument by looking up the online player.
    pub fn get_player(&self, name: &str) -> Result<Player, CommandError> {
        let username = self.get_string(name)?;
        if let Some(ref resolver) = self.player_resolver {
            resolver(username).ok_or_else(|| {
                CommandError::InvalidArgument {
                    argument: name.to_string(),
                    expected: "online player".to_string(),
                    found: username.to_string(),
                }
            })
        } else {
            Err(CommandError::ExecutionFailed("Player resolver not available".to_string()))
        }
    }
}

pub type CommandResult = Result<(), CommandError>;
pub type CommandExecutor = Arc<dyn Fn(&CommandContext) -> Result<(), CommandError> + Send + Sync>;
pub type TabCompleter = Arc<dyn Fn(&CommandContext, &[String]) -> Vec<String> + Send + Sync>;

/// A node in a command tree, representing either a root command, subcommand, or argument sequence.
#[derive(Clone)]
pub struct CommandNode {
    pub name: String,
    pub description: String,
    pub permission: Option<String>,
    pub player_only: bool,
    pub usage: Option<String>,
    pub aliases: Vec<String>,
    pub arguments: Vec<Argument>,
    pub subcommands: Vec<CommandNode>,
    pub executor: Option<CommandExecutor>,
    pub tab_completer: Option<TabCompleter>,
}

impl CommandNode {
    /// Creates a literal subcommand node (e.g. "reload", "set", "list").
    pub fn literal(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
            permission: None,
            player_only: false,
            usage: None,
            aliases: Vec::new(),
            arguments: Vec::new(),
            subcommands: Vec::new(),
            executor: None,
            tab_completer: None,
        }
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    pub fn permission(mut self, permission: impl Into<String>) -> Self {
        self.permission = Some(permission.into());
        self
    }

    /// Constrains this node so only in-game players can execute it.
    pub fn requires_player(mut self) -> Self {
        self.player_only = true;
        self
    }

    pub fn usage(mut self, usage: impl Into<String>) -> Self {
        self.usage = Some(usage.into());
        self
    }

    pub fn alias(mut self, alias: impl Into<String>) -> Self {
        self.aliases.push(alias.into());
        self
    }

    /// Adds a typed argument to this node.
    pub fn argument(mut self, argument: Argument) -> Self {
        self.arguments.push(argument);
        self
    }

    /// Adds a subcommand (literal branch) to this node.
    pub fn subcommand(mut self, node: CommandNode) -> Self {
        self.subcommands.push(node);
        self
    }

    /// Attaches the execution handler for this node.
    pub fn executes<F>(mut self, callback: F) -> Self
    where
        F: Fn(&CommandContext) -> Result<(), CommandError> + Send + Sync + 'static,
    {
        self.executor = Some(Arc::new(callback));
        self
    }

    /// Attaches a custom tab completer for this node.
    pub fn tab_complete<F>(mut self, completer: F) -> Self
    where
        F: Fn(&CommandContext, &[String]) -> Vec<String> + Send + Sync + 'static,
    {
        self.tab_completer = Some(Arc::new(completer));
        self
    }
}

/// High-level builder for registering commands on PotatoMC.
///
/// Supports both Brigadier-style structured trees (`Command::tree(...)`) and
/// Paper-style basic commands (`Command::basic(...)`).
pub struct Command;

impl Command {
    /// Creates a structured command tree with subcommands and typed arguments.
    pub fn tree(name: impl Into<String>) -> CommandNode {
        CommandNode::literal(name)
    }

    /// Creates a simple command with raw arguments, usage, and tab completion (Paper BasicCommand style).
    pub fn basic(name: impl Into<String>) -> CommandNode {
        CommandNode::literal(name)
    }
}
