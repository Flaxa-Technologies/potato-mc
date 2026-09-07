use arc_swap::ArcSwap;
use pumpkin_data::entity::MobCategory;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use tracing::{info, warn};

pub static SPAWNING_CONFIG: LazyLock<ArcSwap<PotatoSpawningConfig>> =
    LazyLock::new(|| ArcSwap::from_pointee(PotatoSpawningConfig::default()));

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PotatoSpawningConfigFile {
    pub spawning: PotatoSpawningConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PotatoSpawningConfig {
    #[serde(default)]
    pub global: GlobalSpawningConfig,
    #[serde(rename = "per-world-caps", default)]
    pub per_world_caps: PerWorldCapsConfig,
    #[serde(default)]
    pub categories: CategoriesConfig,
    #[serde(rename = "per-entity-overrides", default = "default_per_entity_overrides")]
    pub per_entity_overrides: HashMap<String, EntityOverrideConfig>,
    #[serde(default)]
    pub performance: SpawningPerformanceConfig,
}

impl Default for PotatoSpawningConfig {
    fn default() -> Self {
        Self {
            global: GlobalSpawningConfig::default(),
            per_world_caps: PerWorldCapsConfig::default(),
            categories: CategoriesConfig::default(),
            per_entity_overrides: default_per_entity_overrides(),
            performance: SpawningPerformanceConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalSpawningConfig {
    #[serde(rename = "ticks-per-spawn-cycle", default = "default_ticks_per_spawn_cycle")]
    pub ticks_per_spawn_cycle: u32,
    #[serde(rename = "spawn-chunk-radius", default = "default_spawn_chunk_radius")]
    pub spawn_chunk_radius: u32,
    #[serde(rename = "despawn-distance", default = "default_despawn_distance")]
    pub despawn_distance: f64,
    #[serde(rename = "immediate-despawn-range", default = "default_immediate_despawn_range")]
    pub immediate_despawn_range: f64,
}

const fn default_ticks_per_spawn_cycle() -> u32 {
    1
}
const fn default_spawn_chunk_radius() -> u32 {
    8
}
const fn default_despawn_distance() -> f64 {
    128.0
}
const fn default_immediate_despawn_range() -> f64 {
    32.0
}

impl Default for GlobalSpawningConfig {
    fn default() -> Self {
        Self {
            ticks_per_spawn_cycle: default_ticks_per_spawn_cycle(),
            spawn_chunk_radius: default_spawn_chunk_radius(),
            despawn_distance: default_despawn_distance(),
            immediate_despawn_range: default_immediate_despawn_range(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerWorldCapsConfig {
    #[serde(default = "default_monster_cap")]
    pub monster: i32,
    #[serde(default = "default_creature_cap")]
    pub creature: i32,
    #[serde(default = "default_ambient_cap")]
    pub ambient: i32,
    #[serde(rename = "water-creature", default = "default_water_creature_cap")]
    pub water_creature: i32,
    #[serde(rename = "water-ambient", default = "default_water_ambient_cap")]
    pub water_ambient: i32,
    #[serde(rename = "water-underground-creature", default = "default_water_underground_cap")]
    pub water_underground_creature: i32,
    #[serde(default = "default_axolotls_cap")]
    pub axolotls: i32,
}

const fn default_monster_cap() -> i32 {
    70
}
const fn default_creature_cap() -> i32 {
    10
}
const fn default_ambient_cap() -> i32 {
    15
}
const fn default_water_creature_cap() -> i32 {
    5
}
const fn default_water_ambient_cap() -> i32 {
    20
}
const fn default_water_underground_cap() -> i32 {
    5
}
const fn default_axolotls_cap() -> i32 {
    5
}

impl Default for PerWorldCapsConfig {
    fn default() -> Self {
        Self {
            monster: default_monster_cap(),
            creature: default_creature_cap(),
            ambient: default_ambient_cap(),
            water_creature: default_water_creature_cap(),
            water_ambient: default_water_ambient_cap(),
            water_underground_creature: default_water_underground_cap(),
            axolotls: default_axolotls_cap(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CategoriesConfig {
    #[serde(default)]
    pub monster: MonsterCategoryConfig,
    #[serde(default)]
    pub creature: CreatureCategoryConfig,
    #[serde(rename = "water-creature", default)]
    pub water_creature: WaterCreatureCategoryConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonsterCategoryConfig {
    #[serde(rename = "spawn-cost-per-entity", default = "default_spawn_cost")]
    pub spawn_cost_per_entity: f64,
    #[serde(rename = "requires-darkness", default = "default_true")]
    pub requires_darkness: bool,
    #[serde(rename = "min-light-level", default = "default_zero_u8")]
    pub min_light_level: u8,
    #[serde(rename = "max-light-level", default = "default_zero_u8")]
    pub max_light_level: u8,
}

const fn default_spawn_cost() -> f64 {
    1.0
}
const fn default_true() -> bool {
    true
}
const fn default_zero_u8() -> u8 {
    0
}

impl Default for MonsterCategoryConfig {
    fn default() -> Self {
        Self {
            spawn_cost_per_entity: default_spawn_cost(),
            requires_darkness: true,
            min_light_level: 0,
            max_light_level: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatureCategoryConfig {
    #[serde(rename = "spawn-cost-per-entity", default = "default_spawn_cost")]
    pub spawn_cost_per_entity: f64,
    #[serde(rename = "requires-grass-or-valid-block", default = "default_true")]
    pub requires_grass_or_valid_block: bool,
}

impl Default for CreatureCategoryConfig {
    fn default() -> Self {
        Self {
            spawn_cost_per_entity: default_spawn_cost(),
            requires_grass_or_valid_block: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaterCreatureCategoryConfig {
    #[serde(rename = "spawn-cost-per-entity", default = "default_spawn_cost")]
    pub spawn_cost_per_entity: f64,
    #[serde(rename = "requires-water-source", default = "default_true")]
    pub requires_water_source: bool,
}

impl Default for WaterCreatureCategoryConfig {
    fn default() -> Self {
        Self {
            spawn_cost_per_entity: default_spawn_cost(),
            requires_water_source: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityOverrideConfig {
    #[serde(default)]
    pub weight: Option<u32>,
    #[serde(rename = "min-group-size", default)]
    pub min_group_size: Option<i32>,
    #[serde(rename = "max-group-size", default)]
    pub max_group_size: Option<i32>,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

impl Default for EntityOverrideConfig {
    fn default() -> Self {
        Self {
            weight: None,
            min_group_size: None,
            max_group_size: None,
            enabled: true,
        }
    }
}

fn default_per_entity_overrides() -> HashMap<String, EntityOverrideConfig> {
    let mut map = HashMap::new();
    map.insert(
        "minecraft:zombie".to_string(),
        EntityOverrideConfig {
            weight: Some(100),
            min_group_size: Some(1),
            max_group_size: Some(4),
            enabled: true,
        },
    );
    map.insert(
        "minecraft:skeleton".to_string(),
        EntityOverrideConfig {
            weight: Some(100),
            min_group_size: Some(1),
            max_group_size: Some(4),
            enabled: true,
        },
    );
    map.insert(
        "minecraft:cow".to_string(),
        EntityOverrideConfig {
            weight: Some(8),
            min_group_size: Some(4),
            max_group_size: Some(4),
            enabled: true,
        },
    );
    map.insert(
        "minecraft:axolotl".to_string(),
        EntityOverrideConfig {
            weight: Some(10),
            min_group_size: Some(1),
            max_group_size: Some(4),
            enabled: true,
        },
    );
    map
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawningPerformanceConfig {
    #[serde(rename = "async-eligibility-check", default = "default_true")]
    pub async_eligibility_check: bool,
    #[serde(rename = "batch-per-chunk-column", default = "default_true")]
    pub batch_per_chunk_column: bool,
    #[serde(rename = "cache-spawn-eligible-blocks", default = "default_true")]
    pub cache_spawn_eligible_blocks: bool,
    #[serde(
        rename = "max-spawn-attempts-per-chunk-per-cycle",
        default = "default_max_spawn_attempts"
    )]
    pub max_spawn_attempts_per_chunk_per_cycle: usize,
}

const fn default_max_spawn_attempts() -> usize {
    4
}

impl Default for SpawningPerformanceConfig {
    fn default() -> Self {
        Self {
            async_eligibility_check: true,
            batch_per_chunk_column: true,
            cache_spawn_eligible_blocks: true,
            max_spawn_attempts_per_chunk_per_cycle: 4,
        }
    }
}

const DEFAULT_SPAWNING_YML: &str = r#"# spawning.yml — Potato mob spawning configuration
spawning:
  global:
    ticks-per-spawn-cycle: 1          # vanilla: mob spawns attempted every tick per eligible chunk
    spawn-chunk-radius: 8             # radius (in chunks) around each player where spawning is attempted
    despawn-distance: 128             # blocks; mobs beyond this from all players despawn
    immediate-despawn-range: 32       # blocks; below this, normal despawn timer applies instead of instant

  per-world-caps:                     # matches vanilla per-chunk-side-length-of-17x17 area caps
    monster: 70
    creature: 10
    ambient: 15
    water-creature: 5
    water-ambient: 20
    water-underground-creature: 5
    axolotls: 5

  categories:
    monster:
      spawn-cost-per-entity: 1.0      # relative weight against the category cap
      requires-darkness: true
      min-light-level: 0
      max-light-level: 0
    creature:
      spawn-cost-per-entity: 1.0
      requires-grass-or-valid-block: true
    water-creature:
      spawn-cost-per-entity: 1.0
      requires-water-source: true

  per-entity-overrides:               # optional fine-grained tuning, falls back to category defaults
    minecraft:zombie:
      weight: 100
      min-group-size: 1
      max-group-size: 4
      enabled: true
    minecraft:skeleton:
      weight: 100
      min-group-size: 1
      max-group-size: 4
      enabled: true
    minecraft:cow:
      weight: 8
      min-group-size: 4
      max-group-size: 4
      enabled: true
    minecraft:axolotl:
      weight: 10
      min-group-size: 1
      max-group-size: 4
      enabled: true

  performance:
    async-eligibility-check: true     # do biome/light/block eligibility scan off the tick thread
    batch-per-chunk-column: true      # single pass per column instead of per-category re-scan
    cache-spawn-eligible-blocks: true # avoid re-querying block state for the same column repeatedly
    max-spawn-attempts-per-chunk-per-cycle: 4
"#;

pub fn get_config_path(root: &Path) -> PathBuf {
    root.join("spawning.yml")
}

pub fn load_or_create(root: &Path) -> PotatoSpawningConfig {
    let path = get_config_path(root);
    if !path.exists() {
        if let Err(e) = fs::write(&path, DEFAULT_SPAWNING_YML) {
            warn!("[Spawn] Failed to create default spawning.yml: {e}; using in-memory defaults");
        } else {
            info!("[Spawn] Created default spawning.yml mob spawning configuration");
        }
        return PotatoSpawningConfig::default();
    }

    match fs::read_to_string(&path) {
        Ok(content) => match serde_yaml::from_str::<PotatoSpawningConfigFile>(&content) {
            Ok(file) => {
                info!(
                    "[Spawn] Loaded spawning.yml configuration (monster cap: {}, creature cap: {})",
                    file.spawning.per_world_caps.monster, file.spawning.per_world_caps.creature
                );
                file.spawning
            }
            Err(e) => {
                warn!(
                    "[Spawn] Failed to parse spawning.yml: {e}; falling back to vanilla-equivalent defaults"
                );
                PotatoSpawningConfig::default()
            }
        },
        Err(e) => {
            warn!(
                "[Spawn] Failed to read spawning.yml: {e}; falling back to vanilla-equivalent defaults"
            );
            PotatoSpawningConfig::default()
        }
    }
}

pub fn init(root: &Path) {
    let cfg = load_or_create(root);
    SPAWNING_CONFIG.store(std::sync::Arc::new(cfg));
}

pub fn reload(root: &Path) -> bool {
    let path = get_config_path(root);
    if !path.exists() {
        warn!("[Spawn] Reload failed: spawning.yml not found at {:?}", path);
        return false;
    }

    match fs::read_to_string(&path) {
        Ok(content) => match serde_yaml::from_str::<PotatoSpawningConfigFile>(&content) {
            Ok(file) => {
                info!(
                    "[Spawn] Hot-reloaded spawning.yml (monster cap: {}, creature cap: {})",
                    file.spawning.per_world_caps.monster, file.spawning.per_world_caps.creature
                );
                SPAWNING_CONFIG.store(std::sync::Arc::new(file.spawning));
                true
            }
            Err(e) => {
                warn!("[Spawn] Hot-reload error parsing spawning.yml: {e}; retaining current config");
                false
            }
        },
        Err(e) => {
            warn!("[Spawn] Hot-reload error reading spawning.yml: {e}; retaining current config");
            false
        }
    }
}

impl PotatoSpawningConfig {
    #[must_use]
    pub fn get_category_cap(&self, category: &MobCategory) -> i32 {
        match category.id {
            0 => self.per_world_caps.monster,
            1 => self.per_world_caps.creature,
            2 => self.per_world_caps.ambient,
            3 => self.per_world_caps.axolotls,
            4 => self.per_world_caps.water_underground_creature,
            5 => self.per_world_caps.water_creature,
            6 => self.per_world_caps.water_ambient,
            _ => category.max,
        }
    }

    #[must_use]
    pub fn get_entity_override(&self, resource_name: &str) -> Option<&EntityOverrideConfig> {
        let key = if resource_name.contains(':') {
            resource_name.to_string()
        } else {
            format!("minecraft:{resource_name}")
        };
        self.per_entity_overrides.get(&key)
    }

    #[must_use]
    pub fn is_entity_enabled(&self, resource_name: &str) -> bool {
        self.get_entity_override(resource_name)
            .is_none_or(|ovr| ovr.enabled)
    }

    #[must_use]
    pub fn get_entity_weight(&self, resource_name: &str, default_weight: u32) -> u32 {
        self.get_entity_override(resource_name)
            .and_then(|ovr| ovr.weight)
            .unwrap_or(default_weight)
    }

    #[must_use]
    pub fn get_entity_group_bounds(&self, resource_name: &str, min: i32, max: i32) -> (i32, i32) {
        if let Some(ovr) = self.get_entity_override(resource_name) {
            let actual_min = ovr.min_group_size.unwrap_or(min);
            let actual_max = ovr.max_group_size.unwrap_or(max).max(actual_min);
            (actual_min, actual_max)
        } else {
            (min, max)
        }
    }
}
