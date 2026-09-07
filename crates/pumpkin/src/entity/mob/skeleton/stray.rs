use std::sync::Arc;

use crate::entity::{
    Entity,
    mob::{Mob, MobEntity, skeleton::SkeletonEntityBase},
};

pub struct StraySkeletonEntity {
    pub entity: Arc<SkeletonEntityBase>,
}

impl StraySkeletonEntity {
    #[must_use]
    pub fn new(entity: Entity) -> Arc<Self> {
        let entity = SkeletonEntityBase::new(entity);
        let stray = Self { entity };
        Arc::new(stray)
    }
}

impl Mob for StraySkeletonEntity {
    fn get_mob_entity(&self) -> &MobEntity {
        &self.entity.mob_entity
    }

    fn mob_write_nbt(&self, nbt: &mut pumpkin_nbt::compound::NbtCompound) {
        self.entity.mob_write_nbt(nbt);
    }

    fn mob_read_nbt(&self, nbt: &pumpkin_nbt::compound::NbtCompound) {
        self.entity.mob_read_nbt(nbt);
    }
}
