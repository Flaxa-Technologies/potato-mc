use super::{Controls, Goal, to_goal_ticks, track_target::TrackTargetGoal};
use crate::entity::{
    EntityBase,
    mob::Mob,
    passive::villager::VillagerEntity,
};
use rand::RngExt;
use std::sync::Arc;

pub struct DefendVillageTargetGoal {
    track_target_goal: TrackTargetGoal,
    target: Option<Arc<dyn EntityBase>>,
    reciprocal_chance: i32,
}

impl DefendVillageTargetGoal {
    #[must_use]
    pub fn new() -> Self {
        Self {
            track_target_goal: TrackTargetGoal::new(false, false),
            target: None,
            reciprocal_chance: to_goal_ticks(10),
        }
    }

    fn find_target(&mut self, mob: &dyn Mob) -> Option<Arc<dyn EntityBase>> {
        let mob_entity = mob.get_mob_entity();
        let entity = &mob_entity.living_entity.entity;

        // Player-created golems never attack players via DefendVillageTargetGoal
        if let Some(golem) = mob.as_iron_golem() {
            if golem.is_player_created() {
                return None;
            }
        }

        let world = entity.world.load();
        let golem_pos = entity.pos.load();

        // Vanilla DefendVillageTargetGoal: inflate(10.0, 8.0, 10.0) around golem
        // checks villagers within village radius (~16 blocks), and potential targets within 64 blocks
        let village_radius_sq = 16.0 * 16.0;
        let combat_radius_sq = 64.0 * 64.0;

        let entities = world.entities.load();
        for villager_base in entities.iter() {
            if villager_base.get_entity().entity_type != &pumpkin_data::entity::EntityType::VILLAGER {
                continue;
            }

            let v_pos = villager_base.get_entity().pos.load();
            if (v_pos - golem_pos).length_squared() > village_radius_sq {
                continue;
            }

            let Some(villager) = villager_base.cast_any().downcast_ref::<VillagerEntity>() else {
                continue;
            };

            // Check players within 64 blocks of golem with negative reputation (<= -100)
            for player in world.players.load().iter() {
                if player.is_creative() || player.is_spectator() {
                    continue;
                }
                let p_pos = player.living_entity.entity.pos.load();
                if (p_pos - golem_pos).length_squared() > combat_radius_sq {
                    continue;
                }

                let reputation = villager.get_player_reputation(&player.gameprofile.id);
                if reputation <= -100 {
                    return Some(player.clone() as Arc<dyn EntityBase>);
                }
            }
        }

        None
    }
}

impl Default for DefendVillageTargetGoal {
    fn default() -> Self {
        Self::new()
    }
}

impl Goal for DefendVillageTargetGoal {
    fn can_start(&mut self, mob: &dyn Mob) -> bool {
        if self.reciprocal_chance > 0
            && mob.get_random().random_range(0..self.reciprocal_chance) != 0
        {
            return false;
        }

        self.target = self.find_target(mob);
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
        self.target = None;
    }

    fn controls(&self) -> Controls {
        self.track_target_goal.controls()
    }
}
