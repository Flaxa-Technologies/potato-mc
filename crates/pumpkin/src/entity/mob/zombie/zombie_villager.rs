use crate::entity::Entity;
use crate::entity::mob::zombie::ZombieEntityBase;
use crate::entity::mob::{Mob, MobEntity};
use crate::entity::passive::villager::{VillagerData, VillagerEntity};
use crate::entity::player::Player;
use crate::entity::player::advancement::trigger::AdvancementTrigger;
use crossbeam::atomic::AtomicCell;
use pumpkin_data::effect::StatusEffect;
use pumpkin_data::entity::EntityType;
use pumpkin_data::item::Item;
use pumpkin_data::item_stack::ItemStack;
use pumpkin_data::sound::{Sound, SoundCategory};
use pumpkin_data::tracked_data;
use pumpkin_nbt::compound::NbtCompound;
use rustc_hash::FxHashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use uuid::Uuid;

pub struct ZombieVillagerEntity {
    pub mob_entity: Arc<ZombieEntityBase>,
    pub conversion_time: AtomicI32,
    pub conversion_starter: AtomicCell<Option<Uuid>>,
    pub villager_xp: AtomicI32,
    pub villager_data: std::sync::Mutex<VillagerData>,
    pub villager_data_finalized: AtomicBool,
    pub offers: std::sync::Mutex<Vec<pumpkin_protocol::java::client::play::MerchantOffer>>,
    pub gossips: std::sync::Mutex<
        Option<FxHashMap<Uuid, FxHashMap<crate::entity::passive::villager::GossipType, i32>>>,
    >,
}

impl ZombieVillagerEntity {
    pub fn new(entity: Entity) -> Arc<Self> {
        let mob_entity = ZombieEntityBase::new(entity);
        let zombie = Self {
            mob_entity,
            conversion_time: AtomicI32::new(-1),
            conversion_starter: AtomicCell::new(None),
            villager_xp: AtomicI32::new(0),
            villager_data: std::sync::Mutex::new(VillagerData::new(
                crate::entity::passive::villager::data::random_villager_type(),
                crate::entity::passive::villager::data::random_villager_profession(),
                1,
            )),
            villager_data_finalized: AtomicBool::new(false),
            offers: std::sync::Mutex::new(Vec::new()),
            gossips: std::sync::Mutex::new(None),
        };
        Arc::new(zombie)
    }

    #[must_use]
    pub fn with_can_break_doors(entity: Entity, can_break_doors: bool) -> Arc<Self> {
        let mob_entity = ZombieEntityBase::with_can_break_doors(entity, can_break_doors);
        let zombie = Self {
            mob_entity,
            conversion_time: AtomicI32::new(-1),
            conversion_starter: AtomicCell::new(None),
            villager_xp: AtomicI32::new(0),
            villager_data: std::sync::Mutex::new(VillagerData::new(
                crate::entity::passive::villager::data::random_villager_type(),
                crate::entity::passive::villager::data::random_villager_profession(),
                1,
            )),
            villager_data_finalized: AtomicBool::new(false),
            offers: std::sync::Mutex::new(Vec::new()),
            gossips: std::sync::Mutex::new(None),
        };
        Arc::new(zombie)
    }

    pub fn set_villager_data(&self, data: VillagerData) {
        let mut villager_data = self
            .villager_data
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *villager_data = data;
        let entity = &self.mob_entity.mob_entity.living_entity.entity;
        entity.set_synced_data(tracked_data::zombie_villager::DATA_VILLAGER_DATA, data);
    }

    #[must_use]
    pub fn is_baby(&self) -> bool {
        self.mob_entity
            .mob_entity
            .living_entity
            .entity
            .age
            .load(Ordering::Relaxed)
            < 0
    }

    pub fn set_baby(&self, baby: bool) {
        let age = if baby { -24000 } else { 0 };
        let entity = &self.mob_entity.mob_entity.living_entity.entity;
        entity.age.store(age, Ordering::Relaxed);
        entity.set_synced_data(tracked_data::zombie_villager::DATA_BABY_ID, baby);
    }

    #[must_use]
    pub fn is_converting(&self) -> bool {
        self.conversion_time.load(Ordering::Relaxed) > 0
    }

    pub fn start_converting(&self, player: Option<Uuid>, time: i32) {
        self.conversion_starter.store(player);
        self.conversion_time.store(time, Ordering::Relaxed);
        let entity = &self.mob_entity.mob_entity.living_entity.entity;
        entity.set_synced_data(
            tracked_data::zombie_villager::DATA_CONVERTING_ID,
            true,
        );
        let living = &self.mob_entity.mob_entity.living_entity;
        living.remove_effect(&StatusEffect::WEAKNESS);
        living.add_effect(pumpkin_data::potion::Effect {
            effect_type: &StatusEffect::STRENGTH,
            duration: time,
            amplifier: 0,
            ambient: false,
            show_particles: true,
            show_icon: true,
            blend: false,
        });

        let world = entity.world.load();
        let pos = entity.pos.load();
        world.play_sound_fine(
            Sound::EntityZombieVillagerCure,
            SoundCategory::Hostile,
            &pos,
            1.0 + rand::random::<f32>(),
            rand::random::<f32>() * 0.7 + 0.3,
        );
    }

    fn get_conversion_progress(&self) -> i32 {
        let mut amount = 1;
        let entity = &self.mob_entity.mob_entity.living_entity.entity;
        let pos = entity.block_pos.load();
        let world = entity.world.load();
        let mut special_blocks_count = 0;

        'outer: for dx in -4..=4 {
            for dy in -4..=4 {
                for dz in -4..=4 {
                    if special_blocks_count >= 14 {
                        break 'outer;
                    }
                    let check_pos = pos.add(dx, dy, dz);
                    let block = world.get_block(&check_pos);
                    let is_special = block.name == "iron_bars" || block.name.ends_with("_bed");
                    if is_special {
                        if rand::random::<f32>() < 0.3 {
                            amount += 1;
                        }
                        special_blocks_count += 1;
                    }
                }
            }
        }
        amount
    }

    pub fn finish_conversion(&self) {
        let entity = &self.mob_entity.mob_entity.living_entity.entity;
        let world = entity.world.load();
        let pos = entity.pos.load();

        self.conversion_time.store(-1, Ordering::Relaxed);
        entity.set_synced_data(
            tracked_data::zombie_villager::DATA_CONVERTING_ID,
            false,
        );

        // Remove zombie villager
        entity.remove();

        // Spawn converted Villager
        let villager_entity = Entity::new(world.clone(), pos, &EntityType::VILLAGER);
        villager_entity.yaw.store(entity.yaw.load());
        villager_entity.head_yaw.store(entity.head_yaw.load());
        villager_entity.pitch.store(entity.pitch.load());
        villager_entity.velocity.store(entity.velocity.load());

        let villager = VillagerEntity::new(villager_entity);
        let data = *self
            .villager_data
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        villager.set_villager_data(data);
        villager
            .xp
            .store(self.villager_xp.load(Ordering::Relaxed), Ordering::Relaxed);

        let offers = self
            .offers
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        *villager
            .offers
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = offers;

        if let Some(gossips) = self
            .gossips
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
        {
            *villager
                .gossips
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = gossips;
        }

        if self.is_baby() {
            let v_entity = &villager.mob_entity.living_entity.entity;
            v_entity.age.store(-24000, Ordering::Relaxed);
            v_entity.set_synced_data(tracked_data::zombie_villager::DATA_BABY_ID, true);
        }

        world.spawn_entity_non_save(villager.clone() as Arc<dyn crate::entity::EntityBase>);

        if let Some(starter_uuid) = self.conversion_starter.load() {
            {
                let mut gossips = villager
                    .gossips
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                let player_gossips = gossips.entry(starter_uuid).or_default();
                let major = player_gossips
                    .entry(crate::entity::passive::villager::data::GossipType::MajorPositive)
                    .or_default();
                *major = (*major + 20)
                    .min(crate::entity::passive::villager::data::GossipType::MajorPositive.max_value());
                let minor = player_gossips
                    .entry(crate::entity::passive::villager::data::GossipType::MinorPositive)
                    .or_default();
                *minor = (*minor + 25)
                    .min(crate::entity::passive::villager::data::GossipType::MinorPositive.max_value());
            }
            if let Some(player) = world.get_player_by_uuid(starter_uuid) {
                player.trigger_advancement(AdvancementTrigger::CuredZombieVillager);
            }
        }

        world.play_sound_fine(
            Sound::EntityZombieVillagerConverted,
            SoundCategory::Hostile,
            &pos,
            1.0,
            1.0,
        );
    }
}

impl Mob for ZombieVillagerEntity {
    fn get_mob_entity(&self) -> &MobEntity {
        &self.mob_entity.mob_entity
    }

    fn mob_tick(&self, _caller: &dyn crate::entity::EntityBase) {
        if self.is_converting() && self.mob_entity.mob_entity.living_entity.is_alive() {
            let progress = self.get_conversion_progress();
            let current = self.conversion_time.load(Ordering::Relaxed);
            if current <= progress {
                self.finish_conversion();
            } else {
                self.conversion_time.fetch_sub(progress, Ordering::Relaxed);
            }
        }
    }

    fn mob_interact(&self, player: &Arc<Player>, item_stack: &mut ItemStack) -> bool {
        if item_stack.get_item() == &Item::GOLDEN_APPLE {
            let living = &self.mob_entity.mob_entity.living_entity;
            if living.get_effect(&StatusEffect::WEAKNESS).is_some() && !self.is_converting() {
                item_stack.decrement_unless_creative(player.gamemode.load(), 1);
                let time = rand::random_range(3600..=6000);
                self.start_converting(Some(player.gameprofile.id), time);
                return true;
            }
            return true;
        }
        false
    }

    fn remove_when_far_away(&self, _distance_sq: f64) -> bool {
        !self.is_converting() && self.villager_xp.load(Ordering::Relaxed) == 0
    }

    fn mob_set_variant_name(&self, name: &str) {
        let villager_type = crate::entity::passive::villager::data::parse_villager_type(name);
        let current = *self
            .villager_data
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        self.set_villager_data(VillagerData::new(
            villager_type,
            current.profession_enum(),
            current.level.0,
        ));
    }

    fn mob_init_data_tracker(&self) {
        let entity = &self.mob_entity.mob_entity.living_entity.entity;
        let data = *self
            .villager_data
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        entity.set_synced_data(tracked_data::zombie_villager::DATA_VILLAGER_DATA, data);
        if self.is_baby() {
            entity.set_synced_data(tracked_data::zombie_villager::DATA_BABY_ID, true);
        }
        if self.is_converting() {
            entity.set_synced_data(tracked_data::zombie_villager::DATA_CONVERTING_ID, true);
        }
    }

    fn mob_write_nbt(&self, nbt: &mut NbtCompound) {
        self.mob_entity.mob_write_nbt(nbt);
        {
            let data = self
                .villager_data
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let mut villager_data_nbt = NbtCompound::new();
            villager_data_nbt.put_int("Type", data.r#type.0);
            villager_data_nbt.put_int("Profession", data.profession.0);
            villager_data_nbt.put_int("Level", data.level.0);
            nbt.put_compound("VillagerData", villager_data_nbt);
        }
        nbt.put_int(
            "ConversionTime",
            if self.is_converting() {
                self.conversion_time.load(Ordering::Relaxed)
            } else {
                -1
            },
        );
        if let Some(player_uuid) = self.conversion_starter.load() {
            nbt.put(
                "ConversionPlayer",
                pumpkin_nbt::tag::NbtTag::IntArray(vec![
                    (player_uuid.as_u128() >> 96) as i32,
                    (player_uuid.as_u128() >> 64) as i32,
                    (player_uuid.as_u128() >> 32) as i32,
                    player_uuid.as_u128() as i32,
                ]),
            );
        }
        nbt.put_int("Xp", self.villager_xp.load(Ordering::Relaxed));
        nbt.put_bool(
            "VillagerDataFinalized",
            self.villager_data_finalized.load(Ordering::Relaxed),
        );
    }

    fn mob_read_nbt(&self, nbt: &NbtCompound) {
        self.mob_entity.mob_read_nbt(nbt);
        if let Some(villager_data_nbt) = nbt.get_compound("VillagerData") {
            let type_id = villager_data_nbt.get_int("Type").unwrap_or(2);
            let profession_id = villager_data_nbt.get_int("Profession").unwrap_or(0);
            let level = villager_data_nbt.get_int("Level").unwrap_or(1);
            let data = VillagerData {
                r#type: pumpkin_protocol::codec::var_int::VarInt(type_id),
                profession: pumpkin_protocol::codec::var_int::VarInt(profession_id),
                level: pumpkin_protocol::codec::var_int::VarInt(level),
            };
            self.set_villager_data(data);
        }
        let conversion_time = nbt.get_int("ConversionTime").unwrap_or(-1);
        if conversion_time != -1 {
            let conversion_starter = nbt.get_int_array("ConversionPlayer").and_then(|arr| {
                if arr.len() == 4 {
                    let u = ((arr[0] as u128) << 96)
                        | ((arr[1] as u128 & 0xFFFF_FFFF) << 64)
                        | ((arr[2] as u128 & 0xFFFF_FFFF) << 32)
                        | (arr[3] as u128 & 0xFFFF_FFFF);
                    Some(Uuid::from_u128(u))
                } else {
                    None
                }
            });
            self.start_converting(conversion_starter, conversion_time);
        } else {
            self.conversion_time.store(-1, Ordering::Relaxed);
            self.mob_entity.mob_entity.living_entity.entity.set_synced_data(
                tracked_data::zombie_villager::DATA_CONVERTING_ID,
                false,
            );
        }

        self.villager_xp
            .store(nbt.get_int("Xp").unwrap_or(0), Ordering::Relaxed);
        self.villager_data_finalized.store(
            nbt.get_bool("VillagerDataFinalized").unwrap_or(false),
            Ordering::Relaxed,
        );
    }
}
