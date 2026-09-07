use pumpkin_config::lighting::LightingEngineConfig;
use pumpkin_util::math::position::BlockPos;

pub use pumpkin_data::environment_attribute::{Activity, MoonPhase};

use super::World;

const MAX_LIGHT_LEVEL: u8 = 15;

impl World {

    pub fn get_lighting_config(&self) -> LightingEngineConfig {
        self.server
            .upgrade()
            .map(|s| s.advanced_config.world.lighting)
            .unwrap_or_default()
    }


    #[must_use]
    pub fn get_sky_darken(&self) -> i32 {
        let sky_light_level = self.environment_attributes().get_dimension_value_f32(
            pumpkin_data::environment_attribute::EnvironmentAttribute::GameplaySkyLightLevel,
        );
        (15.0 - sky_light_level).clamp(0.0, 15.0) as i32
    }


    #[must_use]
    pub fn is_bright_outside(&self) -> bool {
        !self.dimension.has_fixed_time && self.get_sky_darken() < 4
    }


    #[must_use]
    pub fn is_dark_outside(&self) -> bool {
        !self.dimension.has_fixed_time && !self.is_bright_outside()
    }


    /// Checks if daylight burns undead monsters (`EnvironmentAttributes.MONSTERS_BURN`).
    #[must_use]
    pub fn monsters_burn(&self, pos: &BlockPos) -> bool {
        self.environment_attributes().get_value_bool(
            pumpkin_data::environment_attribute::EnvironmentAttribute::GameplayMonstersBurn,
            pos,
        )
    }


    /// Checks if bees should stay inside beehives/nests (`EnvironmentAttributes.BEES_STAY_IN_HIVE`).
    #[must_use]
    pub fn bees_stay_in_hive(&self, pos: &BlockPos) -> bool {
        self.environment_attributes().get_value_bool(
            pumpkin_data::environment_attribute::EnvironmentAttribute::GameplayBeesStayInHive,
            pos,
        )
    }


    /// Checks if a creaking heart is active (`EnvironmentAttributes.CREAKING_ACTIVE`).
    #[must_use]
    pub fn creaking_active(&self, pos: &BlockPos) -> bool {
        self.environment_attributes().get_value_bool(
            pumpkin_data::environment_attribute::EnvironmentAttribute::GameplayCreakingActive,
            pos,
        )
    }


    /// Checks if an eyeblossom flower should be open (`EnvironmentAttributes.EYEBLOSSOM_OPEN`).
    #[must_use]
    pub fn eyeblossom_open(&self, pos: &BlockPos) -> Option<bool> {
        self.environment_attributes().get_value_tri_state(
            pumpkin_data::environment_attribute::EnvironmentAttribute::GameplayEyeblossomOpen,
            pos,
        )
    }


    #[must_use]
    pub fn get_effective_sky_brightness(&self, pos: &BlockPos) -> i32 {
        let sky_light = self.get_sky_light_level(pos) as i32;
        sky_light - self.get_sky_darken()
    }


    #[must_use]
    pub fn get_sun_angle(&self, pos: &BlockPos) -> f32 {
        let sun_angle_deg = self.environment_attributes().get_value_f32(
            pumpkin_data::environment_attribute::EnvironmentAttribute::VisualSunAngle,
            pos,
        );
        sun_angle_deg * (std::f32::consts::PI / 180.0)
    }


    #[must_use]
    pub fn get_moon_phase(&self) -> MoonPhase {
        self.environment_attributes()
            .get_dimension_value_moon_phase()
    }


    #[must_use]
    pub fn can_pillager_patrol_spawn(&self, pos: &BlockPos) -> bool {
        self.environment_attributes().get_value_bool(
            pumpkin_data::environment_attribute::EnvironmentAttribute::GameplayCanPillagerPatrolSpawn,
            pos,
        )
    }


    #[must_use]
    pub fn surface_slime_spawn_chance(&self, pos: &BlockPos) -> f32 {
        self.environment_attributes().get_value_f32(
            pumpkin_data::environment_attribute::EnvironmentAttribute::GameplaySurfaceSlimeSpawnChance,
            pos,
        )
    }


    #[must_use]
    pub fn villager_activity(&self, pos: &BlockPos, baby: bool) -> Activity {
        self.environment_attributes().get_value_activity(baby, pos)
    }


    pub fn get_max_local_raw_brightness(&self, pos: &BlockPos) -> u8 {
        let sky_light = (self.get_sky_light_level(pos) as i32 - self.get_sky_darken()).max(0) as u8;
        let block_light = self.get_block_light_level(pos).unwrap_or(0);
        sky_light.max(block_light)
    }


    pub fn get_block_light_level(&self, position: &BlockPos) -> Option<u8> {
        self.level
            .light_engine
            .get_block_light_level(&self.level, position)
    }


    pub fn get_sky_light_level(&self, position: &BlockPos) -> u8 {
        self.level
            .light_engine
            .get_sky_light_level(&self.level, position)
    }


    #[must_use]
    pub fn can_see_sky(&self, position: &BlockPos) -> bool {
        position.0.y >= self.dimension.min_y
            && position.0.y < self.dimension.min_y + self.dimension.height
            && self.get_sky_light_level(position) >= MAX_LIGHT_LEVEL
    }


    pub fn set_block_light_level(&self, position: &BlockPos, light_level: u8) {
        let _ = self
            .level
            .light_engine
            .set_block_light_level(&self.level, position, light_level);
    }


    pub fn set_sky_light_level(&self, position: &BlockPos, light_level: u8) {
        let _ = self
            .level
            .light_engine
            .set_sky_light_level(&self.level, position, light_level);
    }

    #[must_use]
    pub fn is_valid(dest: BlockPos) -> bool {
        Self::is_valid_horizontally(dest) && Self::is_valid_vertically(dest.0.y)
    }

    #[must_use]
    pub fn is_valid_horizontally(dest: BlockPos) -> bool {
        // Note: 30_000_000 is not valid, but -30_000_000 is.
        (-30_000_000..30_000_000).contains(&dest.0.x)
            && (-30_000_000..30_000_000).contains(&dest.0.z)
    }

    #[must_use]
    pub fn is_valid_vertically(y: i32) -> bool {
        // Note: 20_000_000 is not valid, but -20_000_000 is.
        (-20_000_000..20_000_000).contains(&y)
    }

    #[must_use]
    pub fn is_in_build_limit(&self, dest: BlockPos) -> bool {
        self.is_in_height_limit(dest.0.y) && Self::is_valid_horizontally(dest)
    }

    #[must_use]
    pub fn is_in_height_limit(&self, y: i32) -> bool {
        (self.get_bottom_y()..=self.get_top_y()).contains(&y)
    }

    pub const fn get_bottom_y(&self) -> i32 {
        self.dimension.min_y
    }

    pub const fn get_top_y(&self) -> i32 {
        self.dimension.min_y + self.dimension.height - 1
    }

}


struct CubicCurve {
    a: f32,
    b: f32,
    c: f32,
}

impl CubicCurve {
    fn new(v1: f32, v2: f32) -> Self {
        Self {
            a: 3.0 * v1 - 3.0 * v2 + 1.0,
            b: -6.0 * v1 + 3.0 * v2,
            c: 3.0 * v1,
        }
    }

    fn sample(&self, t: f32) -> f32 {
        ((self.a * t + self.b) * t + self.c) * t
    }

    fn sample_gradient(&self, t: f32) -> f32 {
        (3.0 * self.a * t + 2.0 * self.b) * t + self.c
    }
}


/// Calculates the celestial (sun) angle fraction in `[0.0, 1.0]`.
/// Matches vanilla 26.2 `EnvironmentAttributes.SUN_ANGLE` easing with `symmetricCubicBezier(0.362, 0.241)`.
#[must_use]
pub fn calculate_celestial_angle(time_of_day: i64) -> f32 {
    let ticks = time_of_day.rem_euclid(24000);
    let alpha = if ticks < 6000 {
        (ticks + 18000) as f32 / 24000.0
    } else {
        (ticks - 6000) as f32 / 24000.0
    };

    let x_curve = CubicCurve::new(0.362, 0.638);
    let y_curve = CubicCurve::new(0.241, 0.759);

    let mut t = alpha;
    let mut solved = false;
    for _ in 0..4 {
        let error = x_curve.sample(t) - alpha;
        if error.abs() < 1e-5 {
            solved = true;
            break;
        }
        let gradient = x_curve.sample_gradient(t);
        if gradient < 1e-5 {
            break;
        }
        t -= (error / gradient).clamp(-0.25, 0.25);
    }

    if !solved {
        let mut t0 = 0.0f32;
        let mut t1 = 1.0f32;
        for _ in 0..64 {
            if t0 >= t1 {
                break;
            }
            let error = x_curve.sample(t) - alpha;
            if error.abs() < 1e-5 {
                break;
            }
            if error < 0.0 {
                t0 = t;
            } else {
                t1 = t;
            }
            t = f32::midpoint(t1, t0);
        }
    }

    y_curve.sample(t)
}

