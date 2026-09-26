use pumpkin_util::{
    math::position::BlockPos,
    random::{RandomGenerator, RandomImpl},
};

use crate::generation::feature::placed_features::PlacedFeatureWrapper;
use crate::generation::proto_chunk::GenerationCache;
use crate::world::WorldPortalExt;

pub struct WeightedFeatureEntry {
    pub feature: PlacedFeatureWrapper,
    pub weight: u32,
}

pub struct WeightedRandomFeature {
    pub features: Vec<WeightedFeatureEntry>,
    pub total_weight: u32,
}

impl WeightedRandomFeature {
    #[must_use]
    pub fn new(features: Vec<WeightedFeatureEntry>) -> Self {
        let total_weight = features.iter().map(|f| f.weight).sum();
        Self {
            features,
            total_weight,
        }
    }

    #[expect(clippy::too_many_arguments)]
    pub fn generate<T: GenerationCache>(
        &self,
        chunk: &mut T,
        block_registry: &dyn WorldPortalExt,
        min_y: i8,
        height: u16,
        feature_name: pumpkin_data::placed_feature::PlacedFeature,
        random: &mut RandomGenerator,
        pos: BlockPos,
    ) -> bool {
        if self.features.is_empty() || self.total_weight == 0 {
            return false;
        }

        let mut roll = random.next_bounded_i32(self.total_weight as i32) as u32;
        for entry in &self.features {
            if roll < entry.weight {
                if let Some(f) = entry.feature.get() {
                    let child_name = match &entry.feature {
                        PlacedFeatureWrapper::Named(n) => *n,
                        PlacedFeatureWrapper::Direct(_) => feature_name,
                    };
                    return f.generate(
                        chunk,
                        block_registry,
                        min_y,
                        height,
                        child_name,
                        random,
                        pos,
                    );
                }
                return false;
            }
            roll -= entry.weight;
        }
        false
    }
}
