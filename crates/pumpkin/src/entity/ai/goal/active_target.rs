use super::{Controls, Goal, to_goal_ticks};

use crate::entity::ai::goal::track_target::TrackTargetGoal;
use crate::entity::ai::target_predicate::TargetPredicate;
use crate::entity::living::LivingEntity;
use crate::entity::mob::Mob;
use crate::entity::{EntityBase, mob::MobEntity};
use crate::world::World;
use pumpkin_data::attributes::Attributes;
use pumpkin_data::entity::EntityType;
use rand::RngExt;
use std::sync::Arc;

const DEFAULT_RECIPROCAL_CHANCE: i32 = 10;

pub struct ActiveTargetGoal {
    track_target_goal: TrackTargetGoal,
    target: Option<Arc<dyn EntityBase>>,
    reciprocal_chance: i32,
    /// The entity type to search for, or `None` to match any non-friendly mob
    /// (equivalent to vanilla's `NearestAttackableTargetGoal<Mob>`).
    target_type: Option<&'static EntityType>,
    target_predicate: TargetPredicate,
}

impl ActiveTargetGoal {
    pub fn new<F>(
        mob: &MobEntity,
        target_type: &'static EntityType,
        reciprocal_chance: i32,
        check_visibility: bool,
        check_can_navigate: bool,
        predicate: Option<F>,
    ) -> Self
    where
        F: Fn(&LivingEntity, &World) -> bool + Send + Sync + 'static,
    {
        let track_target_goal = TrackTargetGoal::new(check_visibility, check_can_navigate);
        let mut target_predicate = TargetPredicate::create_attackable();
        target_predicate.base_max_distance = mob
            .living_entity
            .get_attribute_value(&Attributes::FOLLOW_RANGE);

        if let Some(predicate) = predicate {
            target_predicate.set_predicate(predicate);
        }

        Self {
            track_target_goal,
            target: None,
            reciprocal_chance: to_goal_ticks(reciprocal_chance),
            target_type: Some(target_type),
            target_predicate,
        }
    }

    /// Constructs an `ActiveTargetGoal` that searches **any** non-player living entity
    /// filtered solely by `predicate`. Equivalent to vanilla's
    /// `NearestAttackableTargetGoal<Mob>` (or `<Enemy>`).
    #[must_use]
    pub fn with_mob_class<F>(
        mob: &MobEntity,
        reciprocal_chance: i32,
        check_visibility: bool,
        predicate: F,
    ) -> Box<Self>
    where
        F: Fn(&LivingEntity, &World) -> bool + Send + Sync + 'static,
    {
        let track_target_goal = TrackTargetGoal::new(check_visibility, false);
        let mut target_predicate = TargetPredicate::create_attackable();
        target_predicate.base_max_distance = mob
            .living_entity
            .get_attribute_value(&Attributes::FOLLOW_RANGE);
        target_predicate.set_predicate(predicate);

        Box::new(Self {
            track_target_goal,
            target: None,
            reciprocal_chance: to_goal_ticks(reciprocal_chance),
            target_type: None, // match any living entity; predicate does the filtering
            target_predicate,
        })
    }

    #[must_use]
    pub fn with_default(
        mob: &MobEntity,
        target_type: &'static EntityType,
        check_visibility: bool,
    ) -> Box<Self> {
        let track_target_goal = TrackTargetGoal::with_default(check_visibility);
        let mut target_predicate = TargetPredicate::create_attackable();
        target_predicate.base_max_distance = mob
            .living_entity
            .get_attribute_value(&Attributes::FOLLOW_RANGE);

        Box::new(Self {
            track_target_goal,
            target: None,
            reciprocal_chance: to_goal_ticks(DEFAULT_RECIPROCAL_CHANCE),
            target_type: Some(target_type),
            target_predicate,
        })
    }

    pub fn set_target(&mut self, target: Option<Arc<dyn EntityBase>>) {
        self.target = target;
    }

    fn find_closest_target(&mut self, mob: &MobEntity) {
        let follow_range = mob
            .living_entity
            .get_attribute_value(&Attributes::FOLLOW_RANGE);

        // Vanilla updates the target conditions with the current follow distance on every search
        self.target_predicate.base_max_distance = follow_range;

        let world = mob.living_entity.entity.world.load();

        // Vanilla searches using getEyeY(), so we offset the position by the eye height
        let mut search_pos = mob.living_entity.entity.pos.load();
        search_pos.y += mob.living_entity.entity.entity_dimension.load().eye_height as f64;

        match self.target_type {
            Some(target_type) if target_type == &EntityType::PLAYER => {
                let players = world.get_nearby_players(search_pos, follow_range);
                let mut closest: Option<(Arc<dyn EntityBase>, f64)> = None;
                for player in players {
                    let dist_sq = player
                        .get_entity()
                        .pos
                        .load()
                        .squared_distance_to_vec(&search_pos);
                    if self
                        .target_predicate
                        .test(&world, Some(&mob.living_entity), &player.living_entity)
                        && closest.as_ref().map_or(true, |(_, d)| dist_sq < *d)
                    {
                        closest = Some((player as Arc<dyn EntityBase>, dist_sq));
                    }
                }
                if let Some((target, _)) = closest {
                    self.target = Some(target);
                    return;
                }
            }
            Some(target_type) => {
                let entities = world.get_nearby_entities(search_pos, follow_range);
                let mut closest: Option<(Arc<dyn EntityBase>, f64)> = None;
                for (_, entity) in entities {
                    if entity.get_entity().entity_type == target_type
                        && let Some(living) = entity.get_living_entity()
                    {
                        let dist_sq = entity
                            .get_entity()
                            .pos
                            .load()
                            .squared_distance_to_vec(&search_pos);
                        if self
                            .target_predicate
                            .test(&world, Some(&mob.living_entity), living)
                            && closest.as_ref().map_or(true, |(_, d)| dist_sq < *d)
                        {
                            closest = Some((entity, dist_sq));
                        }
                    }
                }
                if let Some((target, _)) = closest {
                    self.target = Some(target);
                    return;
                }
            }
            None => {
                // Match any nearby living entity — the predicate does type-specific filtering.
                // Equivalent to vanilla's NearestAttackableTargetGoal<Mob>.
                let entities = world.get_nearby_entities(search_pos, follow_range);
                let mut closest: Option<(Arc<dyn EntityBase>, f64)> = None;
                for (_, entity) in entities {
                    if let Some(living) = entity.get_living_entity() {
                        let dist_sq = entity
                            .get_entity()
                            .pos
                            .load()
                            .squared_distance_to_vec(&search_pos);
                        if self
                            .target_predicate
                            .test(&world, Some(&mob.living_entity), living)
                            && closest.as_ref().map_or(true, |(_, d)| dist_sq < *d)
                        {
                            closest = Some((entity, dist_sq));
                        }
                    }
                }
                if let Some((target, _)) = closest {
                    self.target = Some(target);
                    return;
                }
            }
        }
        self.target = None;
    }
}

impl Goal for ActiveTargetGoal {
    fn can_start(&mut self, mob: &dyn Mob) -> bool {
        if self.reciprocal_chance > 0
            && mob.get_random().random_range(0..self.reciprocal_chance) != 0
        {
            return false;
        }
        self.find_closest_target(mob.get_mob_entity());
        self.target.is_some()
    }

    fn should_continue(&self, mob: &dyn Mob) -> bool {
        self.track_target_goal.should_continue(mob)
    }

    fn start(&mut self, mob: &dyn Mob) {
        mob.set_mob_target(self.target.clone());
        self.track_target_goal.start(mob);
    }

    fn stop(&mut self, mob: &dyn Mob) {
        self.track_target_goal.stop(mob);
    }

    fn controls(&self) -> Controls {
        self.track_target_goal.controls()
    }
}
