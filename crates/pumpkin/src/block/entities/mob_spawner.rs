use std::sync::{
    Arc,
    atomic::{AtomicI32, Ordering},
};

use crossbeam::atomic::AtomicCell;
use pumpkin_data::{entity::EntityType, world::WorldEvent};
use pumpkin_nbt::compound::NbtCompound;
use pumpkin_util::math::{position::BlockPos, vector3::Vector3};

use crate::{block::entities::BlockEntity, entity::EntityBase, world::World};

pub struct MobSpawnerBlockEntity {
    pub position: BlockPos,
    pub delay: AtomicI32,
    pub max_delay: i32,
    pub min_delay: i32,
    pub spawn_count: i32,
    pub spawn_range: i32,
    pub max_nearby_entities: i32,
    pub required_player_range: i32,
    pub entity_type: AtomicCell<Option<&'static EntityType>>,
}

impl MobSpawnerBlockEntity {
    pub const ID: &'static str = "minecraft:mob_spawner";
    pub const DEFAULT_ENTITY_TYPE: &'static EntityType = &EntityType::PIG;
    pub const DEFAULT_DELAY: i32 = 20;
    pub const DEFAULT_MAX_SPAWN_DELAY: i32 = 800;
    pub const DEFAULT_MIN_SPAWN_DELAY: i32 = 200;
    pub const DEFAULT_SPAWN_COUNT: i32 = 4;
    pub const DEFAULT_SPAWN_RANGE: i32 = 4;
    pub const DEFAULT_MAX_NEARBY_ENTITIES: i32 = 6;
    pub const DEFAULT_REQUIRED_PLAYER_RANGE: i32 = 16;

    #[must_use]
    pub const fn new(position: BlockPos, entity_type: Option<&'static EntityType>) -> Self {
        let entity_type = match entity_type {
            Some(t) => t,
            None => Self::DEFAULT_ENTITY_TYPE,
        };
        Self {
            position,
            delay: AtomicI32::new(Self::DEFAULT_DELAY),
            max_delay: Self::DEFAULT_MAX_SPAWN_DELAY,
            min_delay: Self::DEFAULT_MIN_SPAWN_DELAY,
            spawn_count: Self::DEFAULT_SPAWN_COUNT,
            spawn_range: Self::DEFAULT_SPAWN_RANGE,
            max_nearby_entities: Self::DEFAULT_MAX_NEARBY_ENTITIES,
            required_player_range: Self::DEFAULT_REQUIRED_PLAYER_RANGE,
            entity_type: AtomicCell::new(Some(entity_type)),
        }
    }

    pub fn write_spawner_nbt(&self, nbt: &mut NbtCompound) {
        nbt.put_short("Delay", self.delay.load(Ordering::Relaxed) as i16);
        nbt.put_short("MinSpawnDelay", self.min_delay as i16);
        nbt.put_short("MaxSpawnDelay", self.max_delay as i16);
        nbt.put_short("SpawnCount", self.spawn_count as i16);
        nbt.put_short("SpawnRange", self.spawn_range as i16);
        nbt.put_short("MaxNearbyEntities", self.max_nearby_entities as i16);
        nbt.put_short("RequiredPlayerRange", self.required_player_range as i16);

        if let Some(entity_type) = self.entity_type.load() {
            let mut spawn_entry = NbtCompound::new();

            let mut entity_nbt = NbtCompound::new();
            entity_nbt.put_string("id", format!("minecraft:{}", entity_type.resource_name));

            spawn_entry.put_compound("entity", entity_nbt);

            nbt.put_compound("SpawnData", spawn_entry.clone());

            let mut spawn_potentials = Vec::new();
            let mut potential_entry = NbtCompound::new();
            potential_entry.put_compound("data", spawn_entry);
            potential_entry.put_int("weight", 1);
            spawn_potentials.push(pumpkin_nbt::tag::NbtTag::Compound(potential_entry));
            nbt.put_list("SpawnPotentials", spawn_potentials);
        }
    }
}

impl MobSpawnerBlockEntity {
    fn update_spawns(&self, world: &Arc<World>) {
        let min_delay = self.min_delay;
        let max_delay = self.max_delay;

        self.delay.store(
            if max_delay <= min_delay {
                min_delay
            } else {
                min_delay + rand::random_range(0..max_delay - min_delay)
            },
            Ordering::Relaxed,
        );
        world.add_synced_block_event(self.position, 1, 0);
    }

    pub fn set_entity_type(&self, entity_type: &'static EntityType) {
        self.entity_type.store(Some(entity_type));
    }
}

impl BlockEntity for MobSpawnerBlockEntity {
    fn resource_location(&self) -> &'static str {
        Self::ID
    }

    fn get_position(&self) -> BlockPos {
        self.position
    }

    fn tick(&self, world: &Arc<World>) {
        if let Some(entity_type) = &self.entity_type.load() {
            let center = self.position.to_centered_f64();
            let max_player_dist_sq = (self.required_player_range as f64).powi(2);
            let player_nearby = world.players.load().iter().any(|p| {
                p.get_entity().pos.load().squared_distance_to_vec(&center) <= max_player_dist_sq
            });

            if !player_nearby {
                return;
            }

            if self.delay.load(Ordering::Relaxed) < 0 {
                self.update_spawns(world);
                return;
            }
            if self.delay.load(Ordering::Relaxed) > 0 {
                self.delay.fetch_sub(1, Ordering::Relaxed);
                return;
            }

            let search_radius_horiz = (self.spawn_range * 2) as f64;
            let search_radius_vert = 4.0;
            let nearby_count = world
                .entities
                .load()
                .iter()
                .filter(|e| {
                    let ent = e.get_entity();
                    if ent.entity_type.id != entity_type.id {
                        return false;
                    }
                    let pos = ent.pos.load();
                    (pos.x - center.x).abs() <= search_radius_horiz
                        && (pos.z - center.z).abs() <= search_radius_horiz
                        && (pos.y - center.y).abs() <= search_radius_vert
                })
                .count();

            if nearby_count as i32 >= self.max_nearby_entities {
                self.update_spawns(world);
                return;
            }

            let spawn_range = self.spawn_range;
            let mut spawned_any = false;
            for _ in 0..self.spawn_count {
                let pos = self.position.0;

                let spawn_pos = Vector3::new(
                    pos.x as f64
                        + (rand::random::<f64>() - rand::random::<f64>()) * spawn_range as f64
                        + 0.5,
                    (pos.y + rand::random_range(0..3) - 1) as f64,
                    pos.z as f64
                        + (rand::random::<f64>() - rand::random::<f64>()) * spawn_range as f64
                        + 0.5,
                );
                if !world.is_space_empty(entity_type.get_spawn_bounding_box(
                    spawn_pos.x,
                    spawn_pos.y,
                    spawn_pos.z,
                )) {
                    continue;
                }
                let entity = crate::entity::r#type::from_type(
                    entity_type,
                    spawn_pos,
                    world,
                    uuid::Uuid::new_v4(),
                );
                let yaw = rand::random::<f32>() * 360.0;
                entity.get_entity().set_rotation(yaw, 0.0);
                world.spawn_entity(entity);
                world.sync_world_event(WorldEvent::ParticlesMobblockSpawn, self.position, 0);
                spawned_any = true;
            }
            if spawned_any {
                self.update_spawns(world);
            } else {
                // If all spawn attempts failed collision checks, reset to a brief retry delay
                // so the spawner doesn't busy-loop every tick with delay == 0.
                self.delay.store(rand::random_range(20..40), Ordering::Relaxed);
            }
        }
    }

    fn from_nbt(nbt: &pumpkin_nbt::compound::NbtCompound, position: BlockPos) -> Self
    where
        Self: Sized,
    {
        let get_num = |name: &str| {
            nbt.get_short(name)
                .map(i32::from)
                .or_else(|| nbt.get_int(name))
                .or_else(|| nbt.get_byte(name).map(i32::from))
        };

        let delay = get_num("Delay").unwrap_or(Self::DEFAULT_DELAY);
        let min_delay = get_num("MinSpawnDelay").unwrap_or(Self::DEFAULT_MIN_SPAWN_DELAY);
        let max_delay = get_num("MaxSpawnDelay").unwrap_or(Self::DEFAULT_MAX_SPAWN_DELAY);
        let spawn_count = get_num("SpawnCount").unwrap_or(Self::DEFAULT_SPAWN_COUNT);
        let spawn_range = get_num("SpawnRange").unwrap_or(Self::DEFAULT_SPAWN_RANGE);
        let max_nearby_entities =
            get_num("MaxNearbyEntities").unwrap_or(Self::DEFAULT_MAX_NEARBY_ENTITIES);
        let required_player_range =
            get_num("RequiredPlayerRange").unwrap_or(Self::DEFAULT_REQUIRED_PLAYER_RANGE);

        let entity_type = nbt
            .get_compound("SpawnData")
            .and_then(|data| {
                data.get_compound("entity")
                    .and_then(|entity| entity.get_string("id"))
                    .or_else(|| data.get_string("id"))
            })
            .or_else(|| {
                nbt.get_list("SpawnPotentials")
                    .and_then(|list| list.first())
                    .and_then(|tag| tag.extract_compound())
                    .and_then(|entry| {
                        entry
                            .get_compound("data")
                            .and_then(|data| {
                                data.get_compound("entity")
                                    .and_then(|entity| entity.get_string("id"))
                                    .or_else(|| data.get_string("id"))
                            })
                            .or_else(|| {
                                entry
                                    .get_compound("entity")
                                    .and_then(|entity| entity.get_string("id"))
                            })
                    })
            })
            .or_else(|| nbt.get_string("EntityId"))
            .and_then(EntityType::from_name)
            .or(Some(Self::DEFAULT_ENTITY_TYPE));

        Self {
            position,
            delay: AtomicI32::new(delay),
            max_delay,
            min_delay,
            spawn_count,
            spawn_range,
            max_nearby_entities,
            required_player_range,
            entity_type: AtomicCell::new(entity_type),
        }
    }

    fn write_nbt(&self, nbt: &mut NbtCompound) {
        self.write_spawner_nbt(nbt);
    }

    fn chunk_data_nbt(&self) -> Option<NbtCompound> {
        let mut final_nbt = NbtCompound::new();
        final_nbt.put_short("Delay", self.delay.load(Ordering::Relaxed) as i16);
        final_nbt.put_short("MinSpawnDelay", self.min_delay as i16);
        final_nbt.put_short("MaxSpawnDelay", self.max_delay as i16);
        final_nbt.put_short("SpawnCount", self.spawn_count as i16);
        final_nbt.put_short("SpawnRange", self.spawn_range as i16);
        final_nbt.put_short("MaxNearbyEntities", self.max_nearby_entities as i16);
        final_nbt.put_short("RequiredPlayerRange", self.required_player_range as i16);

        if let Some(entity_type) = self.entity_type.load() {
            let mut spawn_entry = NbtCompound::new();

            let mut entity_nbt = NbtCompound::new();
            entity_nbt.put_string("id", format!("minecraft:{}", entity_type.resource_name));

            spawn_entry.put_compound("entity", entity_nbt);

            final_nbt.put_compound("SpawnData", spawn_entry);
        }
        Some(final_nbt)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pumpkin_nbt::tag::NbtTag;

    #[test]
    fn test_default_spawner_is_pig() {
        let spawner = MobSpawnerBlockEntity::new(BlockPos::new(0, 64, 0), None);
        assert_eq!(
            spawner.entity_type.load(),
            Some(MobSpawnerBlockEntity::DEFAULT_ENTITY_TYPE)
        );
        assert_eq!(spawner.entity_type.load().unwrap().resource_name, "pig");

        let nbt = spawner
            .chunk_data_nbt()
            .expect("chunk data NBT must be present");
        let spawn_data = nbt
            .get_compound("SpawnData")
            .expect("SpawnData must be present");
        let entity = spawn_data
            .get_compound("entity")
            .expect("entity tag must be present");
        assert_eq!(entity.get_string("id"), Some("minecraft:pig"));
    }

    #[test]
    fn test_from_nbt_empty_defaults_to_pig() {
        let empty_nbt = NbtCompound::new();
        let spawner = MobSpawnerBlockEntity::from_nbt(&empty_nbt, BlockPos::new(0, 64, 0));
        assert_eq!(
            spawner.entity_type.load(),
            Some(MobSpawnerBlockEntity::DEFAULT_ENTITY_TYPE)
        );
        assert_eq!(
            spawner.delay.load(Ordering::Relaxed),
            MobSpawnerBlockEntity::DEFAULT_DELAY
        );
    }

    #[test]
    fn test_from_nbt_spawn_data_skeleton() {
        let mut nbt = NbtCompound::new();
        let mut spawn_data = NbtCompound::new();
        let mut entity = NbtCompound::new();
        entity.put_string("id", "minecraft:skeleton".to_string());
        spawn_data.put_compound("entity", entity);
        nbt.put_compound("SpawnData", spawn_data);

        let spawner = MobSpawnerBlockEntity::from_nbt(&nbt, BlockPos::new(10, 20, 30));
        assert_eq!(
            spawner.entity_type.load().unwrap().resource_name,
            "skeleton"
        );
    }

    #[test]
    fn test_from_nbt_spawn_potentials() {
        let mut nbt = NbtCompound::new();
        let mut potentials = Vec::new();
        let mut entry = NbtCompound::new();
        let mut data = NbtCompound::new();
        let mut entity = NbtCompound::new();
        entity.put_string("id", "minecraft:blaze".to_string());
        data.put_compound("entity", entity);
        entry.put_compound("data", data);
        potentials.push(NbtTag::Compound(entry));
        nbt.put_list("SpawnPotentials", potentials);

        let spawner = MobSpawnerBlockEntity::from_nbt(&nbt, BlockPos::new(0, 64, 0));
        assert_eq!(spawner.entity_type.load().unwrap().resource_name, "blaze");
    }

    #[test]
    fn test_from_nbt_unprefixed_mob_id() {
        let mut nbt = NbtCompound::new();
        let mut spawn_data = NbtCompound::new();
        let mut entity = NbtCompound::new();
        entity.put_string("id", "zombie".to_string());
        spawn_data.put_compound("entity", entity);
        nbt.put_compound("SpawnData", spawn_data);

        let spawner = MobSpawnerBlockEntity::from_nbt(&nbt, BlockPos::new(0, 64, 0));
        assert_eq!(spawner.entity_type.load().unwrap().resource_name, "zombie");
    }

    #[test]
    fn test_set_entity_type() {
        let spawner = MobSpawnerBlockEntity::new(BlockPos::new(0, 64, 0), None);
        assert_eq!(spawner.entity_type.load().unwrap().resource_name, "pig");

        spawner.set_entity_type(&EntityType::ZOMBIE);
        assert_eq!(spawner.entity_type.load().unwrap().resource_name, "zombie");

        let nbt = spawner.chunk_data_nbt().unwrap();
        let spawn_data = nbt.get_compound("SpawnData").unwrap();
        let entity = spawn_data.get_compound("entity").unwrap();
        assert_eq!(entity.get_string("id"), Some("minecraft:zombie"));
    }

    #[test]
    fn test_write_spawner_nbt_preserves_configuration() {
        let spawner =
            MobSpawnerBlockEntity::new(BlockPos::new(0, 64, 0), Some(&EntityType::SPIDER));
        spawner.delay.store(15, Ordering::Relaxed);

        let mut nbt = NbtCompound::new();
        spawner.write_spawner_nbt(&mut nbt);

        assert_eq!(nbt.get_short("Delay"), Some(15));
        assert_eq!(
            nbt.get_short("MinSpawnDelay"),
            Some(MobSpawnerBlockEntity::DEFAULT_MIN_SPAWN_DELAY as i16)
        );
        assert_eq!(
            nbt.get_short("MaxSpawnDelay"),
            Some(MobSpawnerBlockEntity::DEFAULT_MAX_SPAWN_DELAY as i16)
        );

        let spawn_data = nbt.get_compound("SpawnData").unwrap();
        let entity = spawn_data.get_compound("entity").unwrap();
        assert_eq!(entity.get_string("id"), Some("minecraft:spider"));

        let potentials = nbt.get_list("SpawnPotentials").unwrap();
        assert_eq!(potentials.len(), 1);
    }
}
