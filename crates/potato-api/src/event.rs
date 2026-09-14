use crate::entity::{Entity, LivingEntity};
use crate::player::Player;
use crate::npc::NpcClickType;
use crate::types::{Block, EquipmentSlot, GameMode, ItemStack, Location};

/// Event priority in execution order, mirroring Paper/Bukkit EventPriority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EventPriority {
    Lowest = 0,
    Low = 1,
    Normal = 2,
    High = 3,
    Highest = 4,
    Monitor = 5,
}

/// Trait implemented by events that can be prevented / cancelled by plugins.
pub trait Cancellable {
    fn is_cancelled(&self) -> bool;
    fn set_cancelled(&mut self, cancelled: bool);
}

/// Core trait representing an event dispatched by the PotatoMC server.
pub trait Event: 'static + Send + Sync {
    const EVENT_ID: u32;
}

pub trait EventHandler<E: Event>: Send + Sync + 'static {
    fn handle(&self, event: &mut E);
}

impl<E: Event, F: Fn(&mut E) + Send + Sync + 'static> EventHandler<E> for F {
    fn handle(&self, event: &mut E) {
        self(event);
    }
}

/// Action type for player interaction events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    RightClickBlock,
    LeftClickBlock,
    RightClickAir,
    LeftClickAir,
    Physical,
}

pub type InteractAction = Action;

// ==========================================
// 1. PlayerJoinEvent (EVENT_ID = 1)
// ==========================================
#[derive(Clone, Debug)]
pub struct PlayerJoinEvent {
    pub player: Player,
    pub join_message: Option<String>,
    pub cancelled: bool,
}

impl Event for PlayerJoinEvent {
    const EVENT_ID: u32 = 1;
}

impl Cancellable for PlayerJoinEvent {
    fn is_cancelled(&self) -> bool {
        self.cancelled
    }
    fn set_cancelled(&mut self, cancelled: bool) {
        self.cancelled = cancelled;
    }
}

// ==========================================
// 2. PlayerQuitEvent (EVENT_ID = 2)
// ==========================================
#[derive(Clone, Debug)]
pub struct PlayerQuitEvent {
    pub player: Player,
    pub quit_message: Option<String>,
}

impl Event for PlayerQuitEvent {
    const EVENT_ID: u32 = 2;
}

// ==========================================
// 3. PlayerMoveEvent (EVENT_ID = 3)
// ==========================================
#[derive(Clone, Debug)]
pub struct PlayerMoveEvent {
    pub player: Player,
    pub from: Location,
    pub to: Location,
    pub cancelled: bool,
}

impl Event for PlayerMoveEvent {
    const EVENT_ID: u32 = 3;
}

impl Cancellable for PlayerMoveEvent {
    fn is_cancelled(&self) -> bool {
        self.cancelled
    }
    fn set_cancelled(&mut self, cancelled: bool) {
        self.cancelled = cancelled;
    }
}

// ==========================================
// 4. PlayerInteractEvent (EVENT_ID = 4)
// ==========================================
#[derive(Clone, Debug)]
pub struct PlayerInteractEvent {
    pub player: Player,
    pub action: Action,
    pub clicked_block: Option<Block>,
    pub block_pos: Option<(i32, i32, i32)>,
    pub cancelled: bool,
}

impl Event for PlayerInteractEvent {
    const EVENT_ID: u32 = 4;
}

impl Cancellable for PlayerInteractEvent {
    fn is_cancelled(&self) -> bool {
        self.cancelled
    }
    fn set_cancelled(&mut self, cancelled: bool) {
        self.cancelled = cancelled;
    }
}

// ==========================================
// 5. BlockBreakEvent (EVENT_ID = 5)
// ==========================================
#[derive(Clone, Debug)]
pub struct BlockBreakEvent {
    pub player: Option<Player>,
    pub block: Block,
    pub location: Location,
    pub drop_items: bool,
    pub cancelled: bool,
}

impl BlockBreakEvent {
    pub fn drop_items(&self) -> bool {
        self.drop_items
    }

    pub fn set_drop_items(&mut self, drop_items: bool) {
        self.drop_items = drop_items;
    }
}

impl Event for BlockBreakEvent {
    const EVENT_ID: u32 = 5;
}

impl Cancellable for BlockBreakEvent {
    fn is_cancelled(&self) -> bool {
        self.cancelled
    }
    fn set_cancelled(&mut self, cancelled: bool) {
        self.cancelled = cancelled;
    }
}

// ==========================================
// 6. BlockPlaceEvent (EVENT_ID = 6)
// ==========================================
#[derive(Clone, Debug)]
pub struct BlockPlaceEvent {
    pub player: Player,
    pub block: Block,
    pub location: Location,
    pub cancelled: bool,
}

impl Event for BlockPlaceEvent {
    const EVENT_ID: u32 = 6;
}

impl Cancellable for BlockPlaceEvent {
    fn is_cancelled(&self) -> bool {
        self.cancelled
    }
    fn set_cancelled(&mut self, cancelled: bool) {
        self.cancelled = cancelled;
    }
}

// ==========================================
// 7. EntitySpawnEvent (EVENT_ID = 7)
// ==========================================
#[derive(Clone, Debug)]
pub struct EntitySpawnEvent {
    pub entity: Entity,
    pub location: Location,
    pub cancelled: bool,
}

impl Event for EntitySpawnEvent {
    const EVENT_ID: u32 = 7;
}

impl Cancellable for EntitySpawnEvent {
    fn is_cancelled(&self) -> bool {
        self.cancelled
    }
    fn set_cancelled(&mut self, cancelled: bool) {
        self.cancelled = cancelled;
    }
}

// ==========================================
// 8. EntityDeathEvent (EVENT_ID = 8)
// ==========================================
#[derive(Clone, Debug)]
pub struct EntityDeathEvent {
    pub entity: LivingEntity,
    pub killer: Option<Player>,
    pub death_message: Option<String>,
    pub dropped_exp: u32,
}

impl Event for EntityDeathEvent {
    const EVENT_ID: u32 = 8;
}

// ==========================================
// 9. EntityDamageEvent / DamageEvent (EVENT_ID = 9)
// ==========================================
#[derive(Clone, Debug)]
pub struct EntityDamageEvent {
    pub entity: Entity,
    pub damager: Option<Entity>,
    pub damage: f32,
    pub cancelled: bool,
}

impl Event for EntityDamageEvent {
    const EVENT_ID: u32 = 9;
}

impl Cancellable for EntityDamageEvent {
    fn is_cancelled(&self) -> bool {
        self.cancelled
    }
    fn set_cancelled(&mut self, cancelled: bool) {
        self.cancelled = cancelled;
    }
}

pub type DamageEvent = EntityDamageEvent;

// ==========================================
// 10. PlayerChatEvent (EVENT_ID = 10)
// ==========================================
#[derive(Clone, Debug)]
pub struct PlayerChatEvent {
    pub player: Player,
    pub message: String,
    pub cancelled: bool,
}

impl Event for PlayerChatEvent {
    const EVENT_ID: u32 = 10;
}

impl Cancellable for PlayerChatEvent {
    fn is_cancelled(&self) -> bool {
        self.cancelled
    }
    fn set_cancelled(&mut self, cancelled: bool) {
        self.cancelled = cancelled;
    }
}

// ==========================================
// 11. PlayerCommandPreprocessEvent (EVENT_ID = 11)
// ==========================================
#[derive(Clone, Debug)]
pub struct PlayerCommandPreprocessEvent {
    pub player: Player,
    pub command: String,
    pub cancelled: bool,
}

impl Event for PlayerCommandPreprocessEvent {
    const EVENT_ID: u32 = 11;
}

impl Cancellable for PlayerCommandPreprocessEvent {
    fn is_cancelled(&self) -> bool {
        self.cancelled
    }
    fn set_cancelled(&mut self, cancelled: bool) {
        self.cancelled = cancelled;
    }
}

// ==========================================
// 12. PlayerDropItemEvent (EVENT_ID = 12)
// ==========================================
#[derive(Clone, Debug)]
pub struct PlayerDropItemEvent {
    pub player: Player,
    pub item: ItemStack,
    pub cancelled: bool,
}

impl Event for PlayerDropItemEvent {
    const EVENT_ID: u32 = 12;
}

impl Cancellable for PlayerDropItemEvent {
    fn is_cancelled(&self) -> bool {
        self.cancelled
    }
    fn set_cancelled(&mut self, cancelled: bool) {
        self.cancelled = cancelled;
    }
}

// ==========================================
// 13. PlayerItemConsumeEvent (EVENT_ID = 13)
// ==========================================
#[derive(Clone, Debug)]
pub struct PlayerItemConsumeEvent {
    pub player: Player,
    pub item: ItemStack,
    pub cancelled: bool,
}

impl Event for PlayerItemConsumeEvent {
    const EVENT_ID: u32 = 13;
}

impl Cancellable for PlayerItemConsumeEvent {
    fn is_cancelled(&self) -> bool {
        self.cancelled
    }
    fn set_cancelled(&mut self, cancelled: bool) {
        self.cancelled = cancelled;
    }
}

// ==========================================
// 14. PlayerRespawnEvent (EVENT_ID = 14)
// ==========================================
#[derive(Clone, Debug)]
pub struct PlayerRespawnEvent {
    pub player: Player,
    pub respawn_location: Location,
    pub is_bed_spawn: bool,
}

impl Event for PlayerRespawnEvent {
    const EVENT_ID: u32 = 14;
}

// ==========================================
// 15. PlayerTeleportEvent (EVENT_ID = 15)
// ==========================================
#[derive(Clone, Debug)]
pub struct PlayerTeleportEvent {
    pub player: Player,
    pub from: Location,
    pub to: Location,
    pub cancelled: bool,
}

impl Event for PlayerTeleportEvent {
    const EVENT_ID: u32 = 15;
}

impl Cancellable for PlayerTeleportEvent {
    fn is_cancelled(&self) -> bool {
        self.cancelled
    }
    fn set_cancelled(&mut self, cancelled: bool) {
        self.cancelled = cancelled;
    }
}

// ==========================================
// 16. PlayerGameModeChangeEvent (EVENT_ID = 16)
// ==========================================
#[derive(Clone, Debug)]
pub struct PlayerGameModeChangeEvent {
    pub player: Player,
    pub new_gamemode: GameMode,
    pub cancelled: bool,
}

impl Event for PlayerGameModeChangeEvent {
    const EVENT_ID: u32 = 16;
}

impl Cancellable for PlayerGameModeChangeEvent {
    fn is_cancelled(&self) -> bool {
        self.cancelled
    }
    fn set_cancelled(&mut self, cancelled: bool) {
        self.cancelled = cancelled;
    }
}

// ==========================================
// 17. PlayerToggleSneakEvent (EVENT_ID = 17)
// ==========================================
#[derive(Clone, Debug)]
pub struct PlayerToggleSneakEvent {
    pub player: Player,
    pub is_sneaking: bool,
    pub cancelled: bool,
}

impl Event for PlayerToggleSneakEvent {
    const EVENT_ID: u32 = 17;
}

impl Cancellable for PlayerToggleSneakEvent {
    fn is_cancelled(&self) -> bool {
        self.cancelled
    }
    fn set_cancelled(&mut self, cancelled: bool) {
        self.cancelled = cancelled;
    }
}

// ==========================================
// 18. PlayerToggleSprintEvent (EVENT_ID = 18)
// ==========================================
#[derive(Clone, Debug)]
pub struct PlayerToggleSprintEvent {
    pub player: Player,
    pub is_sprinting: bool,
    pub cancelled: bool,
}

impl Event for PlayerToggleSprintEvent {
    const EVENT_ID: u32 = 18;
}

impl Cancellable for PlayerToggleSprintEvent {
    fn is_cancelled(&self) -> bool {
        self.cancelled
    }
    fn set_cancelled(&mut self, cancelled: bool) {
        self.cancelled = cancelled;
    }
}

// ==========================================
// 19. PlayerToggleFlightEvent (EVENT_ID = 19)
// ==========================================
#[derive(Clone, Debug)]
pub struct PlayerToggleFlightEvent {
    pub player: Player,
    pub is_flying: bool,
    pub cancelled: bool,
}

impl Event for PlayerToggleFlightEvent {
    const EVENT_ID: u32 = 19;
}

impl Cancellable for PlayerToggleFlightEvent {
    fn is_cancelled(&self) -> bool {
        self.cancelled
    }
    fn set_cancelled(&mut self, cancelled: bool) {
        self.cancelled = cancelled;
    }
}

// ==========================================
// 20. PlayerItemHeldEvent (EVENT_ID = 20)
// ==========================================
#[derive(Clone, Debug)]
pub struct PlayerItemHeldEvent {
    pub player: Player,
    pub previous_slot: usize,
    pub new_slot: usize,
    pub cancelled: bool,
}

impl Event for PlayerItemHeldEvent {
    const EVENT_ID: u32 = 20;
}

impl Cancellable for PlayerItemHeldEvent {
    fn is_cancelled(&self) -> bool {
        self.cancelled
    }
    fn set_cancelled(&mut self, cancelled: bool) {
        self.cancelled = cancelled;
    }
}

// ==========================================
// 21. InventoryClickEvent (EVENT_ID = 21)
// ==========================================
#[derive(Clone, Debug)]
pub struct InventoryClickEvent {
    pub player: Player,
    pub slot: i32,
    pub click_type: u8,
    pub clicked_item: Option<ItemStack>,
    pub cursor_item: Option<ItemStack>,
    pub cancelled: bool,
}

impl Event for InventoryClickEvent {
    const EVENT_ID: u32 = 21;
}

impl Cancellable for InventoryClickEvent {
    fn is_cancelled(&self) -> bool {
        self.cancelled
    }
    fn set_cancelled(&mut self, cancelled: bool) {
        self.cancelled = cancelled;
    }
}

// ==========================================
// 22. ServerListPingEvent (EVENT_ID = 22)
// ==========================================
#[derive(Clone, Debug)]
pub struct ServerListPingEvent {
    pub motd: String,
    pub online_players: u32,
    pub max_players: u32,
}

impl Event for ServerListPingEvent {
    const EVENT_ID: u32 = 22;
}

// ==========================================
// 23. InventoryOpenEvent (EVENT_ID = 23)
// ==========================================
#[derive(Clone, Debug)]
pub struct InventoryOpenEvent {
    pub player: Player,
    pub title: String,
    pub cancelled: bool,
}

impl Event for InventoryOpenEvent {
    const EVENT_ID: u32 = 23;
}

impl Cancellable for InventoryOpenEvent {
    fn is_cancelled(&self) -> bool {
        self.cancelled
    }
    fn set_cancelled(&mut self, cancelled: bool) {
        self.cancelled = cancelled;
    }
}

// ==========================================
// 24. InventoryCloseEvent (EVENT_ID = 24)
// ==========================================
#[derive(Clone, Debug)]
pub struct InventoryCloseEvent {
    pub player: Player,
    pub title: String,
}

impl Event for InventoryCloseEvent {
    const EVENT_ID: u32 = 24;
}

// ==========================================
// 25. ServerTickStartEvent (EVENT_ID = 25)
// ==========================================
#[derive(Clone, Debug)]
pub struct ServerTickStartEvent {
    pub tick_number: u64,
}

impl Event for ServerTickStartEvent {
    const EVENT_ID: u32 = 25;
}

// ==========================================
// 26. ServerTickEndEvent (EVENT_ID = 26)
// ==========================================
#[derive(Clone, Debug)]
pub struct ServerTickEndEvent {
    pub tick_number: u64,
    pub duration_millis: f64,
}

impl Event for ServerTickEndEvent {
    const EVENT_ID: u32 = 26;
}

// ==========================================
// 27. PlayerPickupItemEvent (EVENT_ID = 27)
// ==========================================
#[derive(Clone, Debug)]
pub struct PlayerPickupItemEvent {
    pub player: Player,
    pub item: ItemStack,
    pub cancelled: bool,
}

impl Event for PlayerPickupItemEvent {
    const EVENT_ID: u32 = 27;
}

impl Cancellable for PlayerPickupItemEvent {
    fn is_cancelled(&self) -> bool {
        self.cancelled
    }
    fn set_cancelled(&mut self, cancelled: bool) {
        self.cancelled = cancelled;
    }
}

// ==========================================
// 28. PlayerDeathEvent (EVENT_ID = 28)
// ==========================================
#[derive(Clone, Debug)]
pub struct PlayerDeathEvent {
    pub player: Player,
    pub drops: Vec<ItemStack>,
    pub dropped_exp: u32,
    pub death_message: Option<String>,
    pub keep_inventory: bool,
    pub keep_level: bool,
}

impl Event for PlayerDeathEvent {
    const EVENT_ID: u32 = 28;
}

// ==========================================
// 29. PlayerLevelChangeEvent (EVENT_ID = 29)
// ==========================================
#[derive(Clone, Debug)]
pub struct PlayerLevelChangeEvent {
    pub player: Player,
    pub old_level: i32,
    pub new_level: i32,
}

impl Event for PlayerLevelChangeEvent {
    const EVENT_ID: u32 = 29;
}

// ==========================================
// 30. PlayerExpChangeEvent (EVENT_ID = 30)
// ==========================================
#[derive(Clone, Debug)]
pub struct PlayerExpChangeEvent {
    pub player: Player,
    pub amount: i32,
}

impl Event for PlayerExpChangeEvent {
    const EVENT_ID: u32 = 30;
}

// ==========================================
// 31. PlayerBedEnterEvent (EVENT_ID = 31)
// ==========================================
#[derive(Clone, Debug)]
pub struct PlayerBedEnterEvent {
    pub player: Player,
    pub bed_location: Location,
    pub cancelled: bool,
}

impl Event for PlayerBedEnterEvent {
    const EVENT_ID: u32 = 31;
}

impl Cancellable for PlayerBedEnterEvent {
    fn is_cancelled(&self) -> bool {
        self.cancelled
    }
    fn set_cancelled(&mut self, cancelled: bool) {
        self.cancelled = cancelled;
    }
}

// ==========================================
// 32. PlayerBedLeaveEvent (EVENT_ID = 32)
// ==========================================
#[derive(Clone, Debug)]
pub struct PlayerBedLeaveEvent {
    pub player: Player,
    pub bed_location: Location,
}

impl Event for PlayerBedLeaveEvent {
    const EVENT_ID: u32 = 32;
}

// ==========================================
// 33. SignChangeEvent (EVENT_ID = 33)
// ==========================================
#[derive(Clone, Debug)]
pub struct SignChangeEvent {
    pub player: Player,
    pub location: Location,
    pub lines: [String; 4],
    pub cancelled: bool,
}

impl Event for SignChangeEvent {
    const EVENT_ID: u32 = 33;
}

impl Cancellable for SignChangeEvent {
    fn is_cancelled(&self) -> bool {
        self.cancelled
    }
    fn set_cancelled(&mut self, cancelled: bool) {
        self.cancelled = cancelled;
    }
}

// ==========================================
// 34. ServerCommandEvent (EVENT_ID = 34)
// ==========================================
#[derive(Clone, Debug)]
pub struct ServerCommandEvent {
    pub sender: String,
    pub command: String,
    pub cancelled: bool,
}

impl Event for ServerCommandEvent {
    const EVENT_ID: u32 = 34;
}

impl Cancellable for ServerCommandEvent {
    fn is_cancelled(&self) -> bool {
        self.cancelled
    }
    fn set_cancelled(&mut self, cancelled: bool) {
        self.cancelled = cancelled;
    }
}

// ==========================================
// 35. WeatherChangeEvent (EVENT_ID = 35)
// ==========================================
#[derive(Clone, Debug)]
pub struct WeatherChangeEvent {
    pub world: String,
    pub to_weather_state: bool,
    pub cancelled: bool,
}

impl Event for WeatherChangeEvent {
    const EVENT_ID: u32 = 35;
}

impl Cancellable for WeatherChangeEvent {
    fn is_cancelled(&self) -> bool {
        self.cancelled
    }
    fn set_cancelled(&mut self, cancelled: bool) {
        self.cancelled = cancelled;
    }
}

// ==========================================
// 36. ThunderChangeEvent (EVENT_ID = 36)
// ==========================================
#[derive(Clone, Debug)]
pub struct ThunderChangeEvent {
    pub world: String,
    pub to_thunder_state: bool,
    pub cancelled: bool,
}

impl Event for ThunderChangeEvent {
    const EVENT_ID: u32 = 36;
}

impl Cancellable for ThunderChangeEvent {
    fn is_cancelled(&self) -> bool {
        self.cancelled
    }
    fn set_cancelled(&mut self, cancelled: bool) {
        self.cancelled = cancelled;
    }
}

// ==========================================
// 37. ExplosionEvent (EVENT_ID = 37)
// ==========================================
#[derive(Clone, Debug)]
pub struct ExplosionEvent {
    pub location: Location,
    pub yield_rate: f32,
    pub cancelled: bool,
}

impl Event for ExplosionEvent {
    const EVENT_ID: u32 = 37;
}

impl Cancellable for ExplosionEvent {
    fn is_cancelled(&self) -> bool {
        self.cancelled
    }
    fn set_cancelled(&mut self, cancelled: bool) {
        self.cancelled = cancelled;
    }
}

// ==========================================
// 38. ProjectileLaunchEvent (EVENT_ID = 38)
// ==========================================
#[derive(Clone, Debug)]
pub struct ProjectileLaunchEvent {
    pub entity: Entity,
    pub shooter: Option<Entity>,
    pub cancelled: bool,
}

impl Event for ProjectileLaunchEvent {
    const EVENT_ID: u32 = 38;
}

impl Cancellable for ProjectileLaunchEvent {
    fn is_cancelled(&self) -> bool {
        self.cancelled
    }
    fn set_cancelled(&mut self, cancelled: bool) {
        self.cancelled = cancelled;
    }
}

// ==========================================
// 39. ProjectileHitEvent (EVENT_ID = 39)
// ==========================================
#[derive(Clone, Debug)]
pub struct ProjectileHitEvent {
    pub entity: Entity,
    pub hit_entity: Option<Entity>,
    pub hit_block: Option<Block>,
    pub cancelled: bool,
}

impl Event for ProjectileHitEvent {
    const EVENT_ID: u32 = 39;
}

impl Cancellable for ProjectileHitEvent {
    fn is_cancelled(&self) -> bool {
        self.cancelled
    }
    fn set_cancelled(&mut self, cancelled: bool) {
        self.cancelled = cancelled;
    }
}

// ==========================================
// 40. EntityTargetEvent (EVENT_ID = 40)
// ==========================================
#[derive(Clone, Debug)]
pub struct EntityTargetEvent {
    pub entity: Entity,
    pub target: Option<Entity>,
    pub cancelled: bool,
}

impl Event for EntityTargetEvent {
    const EVENT_ID: u32 = 40;
}

impl Cancellable for EntityTargetEvent {
    fn is_cancelled(&self) -> bool {
        self.cancelled
    }
    fn set_cancelled(&mut self, cancelled: bool) {
        self.cancelled = cancelled;
    }
}

// ==========================================
// 41. DialogShowEvent (EVENT_ID = 41)
// ==========================================
#[derive(Clone, Debug)]
pub struct DialogShowEvent {
    pub player: Player,
    pub dialog_id: String,
    pub cancelled: bool,
}

impl Event for DialogShowEvent {
    const EVENT_ID: u32 = 41;
}

impl Cancellable for DialogShowEvent {
    fn is_cancelled(&self) -> bool {
        self.cancelled
    }
    fn set_cancelled(&mut self, cancelled: bool) {
        self.cancelled = cancelled;
    }
}

// ==========================================
// 42. DialogClickActionEvent (EVENT_ID = 42)
// ==========================================
#[derive(Clone, Debug)]
pub struct DialogClickActionEvent {
    pub player: Player,
    pub action_id: String,
    pub payload: Option<Vec<u8>>,
    pub cancelled: bool,
}

impl Event for DialogClickActionEvent {
    const EVENT_ID: u32 = 42;
}

impl Cancellable for DialogClickActionEvent {
    fn is_cancelled(&self) -> bool {
        self.cancelled
    }
    fn set_cancelled(&mut self, cancelled: bool) {
        self.cancelled = cancelled;
    }
}

// ==========================================
// 43. DialogClearEvent (EVENT_ID = 43)
// ==========================================
#[derive(Clone, Debug)]
pub struct DialogClearEvent {
    pub player: Player,
}

impl Event for DialogClearEvent {
    const EVENT_ID: u32 = 43;
}

// ==========================================
// 44. PlayerFormResponseEvent (EVENT_ID = 44)
// ==========================================
#[derive(Clone, Debug)]
pub struct PlayerFormResponseEvent {
    pub player: Player,
    pub form_id: u32,
    pub response_json: String,
    pub cancelled: bool,
}

impl Event for PlayerFormResponseEvent {
    const EVENT_ID: u32 = 44;
}

impl Cancellable for PlayerFormResponseEvent {
    fn is_cancelled(&self) -> bool {
        self.cancelled
    }
    fn set_cancelled(&mut self, cancelled: bool) {
        self.cancelled = cancelled;
    }
}

// ==========================================
// 45. PlayerPortalEvent (EVENT_ID = 45)
// ==========================================
#[derive(Clone, Debug)]
pub struct PlayerPortalEvent {
    pub player: Player,
    pub from: Location,
    pub to: Option<Location>,
    pub cancelled: bool,
}

impl Event for PlayerPortalEvent {
    const EVENT_ID: u32 = 45;
}

impl Cancellable for PlayerPortalEvent {
    fn is_cancelled(&self) -> bool {
        self.cancelled
    }
    fn set_cancelled(&mut self, cancelled: bool) {
        self.cancelled = cancelled;
    }
}

// ==========================================
// 46. PlayerItemBreakEvent (EVENT_ID = 46)
// ==========================================
#[derive(Clone, Debug)]
pub struct PlayerItemBreakEvent {
    pub player: Player,
    pub broken_item: ItemStack,
}

impl Event for PlayerItemBreakEvent {
    const EVENT_ID: u32 = 46;
}

// ==========================================
// 47. PlayerBucketEmptyEvent (EVENT_ID = 47)
// ==========================================
#[derive(Clone, Debug)]
pub struct PlayerBucketEmptyEvent {
    pub player: Player,
    pub block_clicked: Block,
    pub bucket_item: ItemStack,
    pub cancelled: bool,
}

impl Event for PlayerBucketEmptyEvent {
    const EVENT_ID: u32 = 47;
}

impl Cancellable for PlayerBucketEmptyEvent {
    fn is_cancelled(&self) -> bool {
        self.cancelled
    }
    fn set_cancelled(&mut self, cancelled: bool) {
        self.cancelled = cancelled;
    }
}

// ==========================================
// 48. PlayerBucketFillEvent (EVENT_ID = 48)
// ==========================================
#[derive(Clone, Debug)]
pub struct PlayerBucketFillEvent {
    pub player: Player,
    pub block_clicked: Block,
    pub bucket_item: ItemStack,
    pub cancelled: bool,
}

impl Event for PlayerBucketFillEvent {
    const EVENT_ID: u32 = 48;
}

impl Cancellable for PlayerBucketFillEvent {
    fn is_cancelled(&self) -> bool {
        self.cancelled
    }
    fn set_cancelled(&mut self, cancelled: bool) {
        self.cancelled = cancelled;
    }
}

// ==========================================
// 49. PlayerShearEntityEvent (EVENT_ID = 49)
// ==========================================
#[derive(Clone, Debug)]
pub struct PlayerShearEntityEvent {
    pub player: Player,
    pub entity: Entity,
    pub item: ItemStack,
    pub cancelled: bool,
}

impl Event for PlayerShearEntityEvent {
    const EVENT_ID: u32 = 49;
}

impl Cancellable for PlayerShearEntityEvent {
    fn is_cancelled(&self) -> bool {
        self.cancelled
    }
    fn set_cancelled(&mut self, cancelled: bool) {
        self.cancelled = cancelled;
    }
}

// ==========================================
// 50. EntityDamageByBlockEvent (EVENT_ID = 50)
// ==========================================
#[derive(Clone, Debug)]
pub struct EntityDamageByBlockEvent {
    pub entity: Entity,
    pub damager_block: Option<Block>,
    pub damage: f32,
    pub cancelled: bool,
}

impl Event for EntityDamageByBlockEvent {
    const EVENT_ID: u32 = 50;
}

impl Cancellable for EntityDamageByBlockEvent {
    fn is_cancelled(&self) -> bool {
        self.cancelled
    }
    fn set_cancelled(&mut self, cancelled: bool) {
        self.cancelled = cancelled;
    }
}

// ==========================================
// 51. EntityCombustEvent (EVENT_ID = 51)
// ==========================================
#[derive(Clone, Debug)]
pub struct EntityCombustEvent {
    pub entity: Entity,
    pub duration_secs: u32,
    pub cancelled: bool,
}

impl Event for EntityCombustEvent {
    const EVENT_ID: u32 = 51;
}

impl Cancellable for EntityCombustEvent {
    fn is_cancelled(&self) -> bool {
        self.cancelled
    }
    fn set_cancelled(&mut self, cancelled: bool) {
        self.cancelled = cancelled;
    }
}

// ==========================================
// 52. EntityCombustByEntityEvent (EVENT_ID = 52)
// ==========================================
#[derive(Clone, Debug)]
pub struct EntityCombustByEntityEvent {
    pub entity: Entity,
    pub combuster: Entity,
    pub duration_secs: u32,
    pub cancelled: bool,
}

impl Event for EntityCombustByEntityEvent {
    const EVENT_ID: u32 = 52;
}

impl Cancellable for EntityCombustByEntityEvent {
    fn is_cancelled(&self) -> bool {
        self.cancelled
    }
    fn set_cancelled(&mut self, cancelled: bool) {
        self.cancelled = cancelled;
    }
}

// ==========================================
// 53. PlayerAdvancementDoneEvent (EVENT_ID = 53)
// ==========================================
#[derive(Clone, Debug)]
pub struct PlayerAdvancementDoneEvent {
    pub player: Player,
    pub advancement_id: String,
}

impl Event for PlayerAdvancementDoneEvent {
    const EVENT_ID: u32 = 53;
}

// ==========================================
// 54. InventoryMoveItemEvent (EVENT_ID = 54)
// ==========================================
#[derive(Clone, Debug)]
pub struct InventoryMoveItemEvent {
    pub source_slot: usize,
    pub destination_slot: usize,
    pub item: ItemStack,
    pub cancelled: bool,
}

impl Event for InventoryMoveItemEvent {
    const EVENT_ID: u32 = 54;
}

impl Cancellable for InventoryMoveItemEvent {
    fn is_cancelled(&self) -> bool {
        self.cancelled
    }
    fn set_cancelled(&mut self, cancelled: bool) {
        self.cancelled = cancelled;
    }
}

// ==========================================
// 55. ServerBroadcastEvent (EVENT_ID = 55)
// ==========================================
#[derive(Clone, Debug)]
pub struct ServerBroadcastEvent {
    pub message: String,
    pub cancelled: bool,
}

impl Event for ServerBroadcastEvent {
    const EVENT_ID: u32 = 55;
}

impl Cancellable for ServerBroadcastEvent {
    fn is_cancelled(&self) -> bool {
        self.cancelled
    }
    fn set_cancelled(&mut self, cancelled: bool) {
        self.cancelled = cancelled;
    }
}

// ==========================================
// 56. ScoreboardScoreChangeEvent (EVENT_ID = 56)
// ==========================================
#[derive(Clone, Debug)]
pub struct ScoreboardScoreChangeEvent {
    pub scoreboard_name: String,
    pub objective_name: String,
    pub entry: String,
    pub previous_score: Option<i32>,
    pub new_score: i32,
    pub cancelled: bool,
}

impl Event for ScoreboardScoreChangeEvent {
    const EVENT_ID: u32 = 56;
}

impl Cancellable for ScoreboardScoreChangeEvent {
    fn is_cancelled(&self) -> bool {
        self.cancelled
    }
    fn set_cancelled(&mut self, cancelled: bool) {
        self.cancelled = cancelled;
    }
}

// ==========================================
// 57. NpcInteractEvent (EVENT_ID = 57)
// ==========================================
#[derive(Clone, Debug)]
pub struct NpcInteractEvent {
    pub player: Player,
    pub npc_id: u32,
    pub click_type: NpcClickType,
    pub hand: EquipmentSlot,
    pub cancelled: bool,
}

impl Event for NpcInteractEvent {
    const EVENT_ID: u32 = 57;
}

impl Cancellable for NpcInteractEvent {
    fn is_cancelled(&self) -> bool {
        self.cancelled
    }
    fn set_cancelled(&mut self, cancelled: bool) {
        self.cancelled = cancelled;
    }
}

// ==========================================
// 58. PlayerMaceSmashEvent (EVENT_ID = 58)
// ==========================================
#[derive(Clone, Debug)]
pub struct PlayerMaceSmashEvent {
    pub player: Player,
    pub target: Entity,
    pub fall_distance: f32,
    pub damage: f32,
    pub cancelled: bool,
}

impl Event for PlayerMaceSmashEvent {
    const EVENT_ID: u32 = 58;
}

impl Cancellable for PlayerMaceSmashEvent {
    fn is_cancelled(&self) -> bool {
        self.cancelled
    }
    fn set_cancelled(&mut self, cancelled: bool) {
        self.cancelled = cancelled;
    }
}

// ==========================================
// 59. WindChargeDetonateEvent (EVENT_ID = 59)
// ==========================================
#[derive(Clone, Debug)]
pub struct WindChargeDetonateEvent {
    pub shooter: Option<Player>,
    pub location: Location,
    pub radius: f32,
    pub knockback: f32,
    pub cancelled: bool,
}

impl Event for WindChargeDetonateEvent {
    const EVENT_ID: u32 = 59;
}

impl Cancellable for WindChargeDetonateEvent {
    fn is_cancelled(&self) -> bool {
        self.cancelled
    }
    fn set_cancelled(&mut self, cancelled: bool) {
        self.cancelled = cancelled;
    }
}




