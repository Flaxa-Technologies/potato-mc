use std::sync::Weak;
use std::sync::atomic::Ordering;
use pumpkin_data::data_component_impl::EquipmentSlot;
use pumpkin_data::item_stack::ItemStack;

use super::{Controls, Goal};
use crate::entity::{
    EntityBase,
    mob::Mob,
    passive::villager::VillagerEntity,
};

/// Vanilla `ShowTradesToPlayer`:
/// When a player is within 4.12 blocks (distance squared <= 17.0) holding an item that matches
/// any trade cost of the villager, the villager holds out the result item in its main hand
/// and cycles every 40 ticks between matching trades.
pub struct ShowTradesToPlayerGoal {
    villager: Weak<VillagerEntity>,
    cycle_counter: i32,
    display_index: usize,
    matching_items: Vec<ItemStack>,
    last_held_item_id: Option<u16>,
    is_showing: bool,
}

impl ShowTradesToPlayerGoal {
    #[must_use]
    pub const fn new(villager: Weak<VillagerEntity>) -> Self {
        Self {
            villager,
            cycle_counter: 0,
            display_index: 0,
            matching_items: Vec::new(),
            last_held_item_id: None,
            is_showing: false,
        }
    }

    fn clear_display(&mut self, villager: &VillagerEntity) {
        if self.is_showing {
            if let Ok(mut equip) = villager.mob_entity.living_entity.entity_equipment.try_lock() {
                equip.put(&EquipmentSlot::MAIN_HAND, ItemStack::EMPTY.clone());
                drop(equip);
                villager
                    .mob_entity
                    .living_entity
                    .send_equipment_changes(&[(EquipmentSlot::MAIN_HAND, ItemStack::EMPTY.clone())]);
            }
            self.is_showing = false;
        }
        self.matching_items.clear();
        self.last_held_item_id = None;
        self.cycle_counter = 0;
        self.display_index = 0;
    }
}

impl Goal for ShowTradesToPlayerGoal {
    fn can_start(&mut self, _mob: &dyn Mob) -> bool {
        let Some(villager) = self.villager.upgrade() else {
            return false;
        };
        let entity = &villager.mob_entity.living_entity.entity;
        if !entity.is_alive() || entity.age.load(Ordering::Relaxed) < 0 {
            return false;
        }

        let world = entity.world.load();
        let center = entity.pos.load();
        let players = world.players.load();
        players.iter().any(|player| {
            let p_ent = player.get_entity();
            p_ent.is_alive() && p_ent.pos.load().squared_distance_to_vec(&center) <= 17.0
        })
    }

    fn should_continue(&self, _mob: &dyn Mob) -> bool {
        let Some(villager) = self.villager.upgrade() else {
            return false;
        };
        let entity = &villager.mob_entity.living_entity.entity;
        if !entity.is_alive() || entity.age.load(Ordering::Relaxed) < 0 {
            return false;
        }

        let world = entity.world.load();
        let center = entity.pos.load();
        let players = world.players.load();
        players.iter().any(|player| {
            let p_ent = player.get_entity();
            p_ent.is_alive() && p_ent.pos.load().squared_distance_to_vec(&center) <= 17.0
        })
    }

    fn stop(&mut self, _mob: &dyn Mob) {
        if let Some(villager) = self.villager.upgrade() {
            self.clear_display(&villager);
        }
    }

    fn tick(&mut self, _mob: &dyn Mob) {
        let Some(villager) = self.villager.upgrade() else {
            return;
        };
        let entity = &villager.mob_entity.living_entity.entity;
        if !entity.is_alive() || entity.age.load(Ordering::Relaxed) < 0 {
            self.clear_display(&villager);
            return;
        }

        let world = entity.world.load();
        let center = entity.pos.load();
        let players = world.players.load();

        // Find closest player within 17.0 distance squared
        let closest_player = players
            .iter()
            .filter(|p| {
                let p_ent = p.get_entity();
                p_ent.is_alive() && p_ent.pos.load().squared_distance_to_vec(&center) <= 17.0
            })
            .min_by(|a, b| {
                let da = a.get_entity().pos.load().squared_distance_to_vec(&center);
                let db = b.get_entity().pos.load().squared_distance_to_vec(&center);
                da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
            });

        let Some(player) = closest_player else {
            self.clear_display(&villager);
            return;
        };

        let player_item = player
            .living_entity
            .entity_equipment
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&EquipmentSlot::MAIN_HAND);

        if player_item.is_empty() {
            self.clear_display(&villager);
            return;
        }

        let current_item_id = player_item.item.id;
        if self.last_held_item_id != Some(current_item_id) {
            self.last_held_item_id = Some(current_item_id);
            self.matching_items.clear();
            let offers = villager
                .offers
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            for offer in offers.iter() {
                if !offer.is_out_of_stock() {
                    let matches_a = offer.base_cost_a.0.item.id == current_item_id;
                    let matches_b = offer
                        .cost_b
                        .as_ref()
                        .is_some_and(|b| b.0.item.id == current_item_id);
                    if matches_a || matches_b {
                        self.matching_items.push(offer.output.0.as_ref().clone());
                    }
                }
            }
            self.display_index = 0;
            self.cycle_counter = 0;

            if let Some(first) = self.matching_items.first() {
                if let Ok(mut equip) = villager.mob_entity.living_entity.entity_equipment.try_lock() {
                    equip.put(&EquipmentSlot::MAIN_HAND, first.clone());
                    drop(equip);
                    villager
                        .mob_entity
                        .living_entity
                        .send_equipment_changes(&[(EquipmentSlot::MAIN_HAND, first.clone())]);
                    self.is_showing = true;
                }
            } else {
                self.clear_display(&villager);
            }
        }

        if self.matching_items.len() >= 2 {
            self.cycle_counter += 1;
            if self.cycle_counter >= 40 {
                self.cycle_counter = 0;
                self.display_index = (self.display_index + 1) % self.matching_items.len();
                let item = &self.matching_items[self.display_index];
                if let Ok(mut equip) = villager.mob_entity.living_entity.entity_equipment.try_lock() {
                    equip.put(&EquipmentSlot::MAIN_HAND, item.clone());
                    drop(equip);
                    villager
                        .mob_entity
                        .living_entity
                        .send_equipment_changes(&[(EquipmentSlot::MAIN_HAND, item.clone())]);
                }
            }
        }

        if self.is_showing {
            let p_ent = player.get_entity();
            let p_pos = p_ent.pos.load();
            villager
                .mob_entity
                .look_control
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .look_at(villager.as_ref(), p_pos.x, p_ent.get_eye_y(), p_pos.z);
        }
    }

    fn should_run_every_tick(&self) -> bool {
        true
    }

    fn controls(&self) -> Controls {
        Controls::LOOK
    }
}
