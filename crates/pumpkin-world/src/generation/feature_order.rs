use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::LazyLock;

use pumpkin_data::chunk::Biome;
use pumpkin_data::placed_feature::PlacedFeature;

// The order of biomes in this array is used by FeatureSorter to break ties between otherwise independent features.
const OVERWORLD_BIOMES: &[&Biome] = &[
    &Biome::MUSHROOM_FIELDS,
    &Biome::DEEP_FROZEN_OCEAN,
    &Biome::FROZEN_OCEAN,
    &Biome::DEEP_COLD_OCEAN,
    &Biome::COLD_OCEAN,
    &Biome::DEEP_OCEAN,
    &Biome::OCEAN,
    &Biome::DEEP_LUKEWARM_OCEAN,
    &Biome::LUKEWARM_OCEAN,
    &Biome::WARM_OCEAN,
    &Biome::STONY_SHORE,
    &Biome::SWAMP,
    &Biome::MANGROVE_SWAMP,
    &Biome::SNOWY_SLOPES,
    &Biome::SNOWY_PLAINS,
    &Biome::SNOWY_BEACH,
    &Biome::WINDSWEPT_GRAVELLY_HILLS,
    &Biome::GROVE,
    &Biome::WINDSWEPT_HILLS,
    &Biome::SNOWY_TAIGA,
    &Biome::WINDSWEPT_FOREST,
    &Biome::TAIGA,
    &Biome::PLAINS,
    &Biome::MEADOW,
    &Biome::BEACH,
    &Biome::FOREST,
    &Biome::OLD_GROWTH_SPRUCE_TAIGA,
    &Biome::FLOWER_FOREST,
    &Biome::BIRCH_FOREST,
    &Biome::DARK_FOREST,
    &Biome::PALE_GARDEN,
    &Biome::SAVANNA_PLATEAU,
    &Biome::SAVANNA,
    &Biome::JUNGLE,
    &Biome::BADLANDS,
    &Biome::DESERT,
    &Biome::WOODED_BADLANDS,
    &Biome::JAGGED_PEAKS,
    &Biome::STONY_PEAKS,
    &Biome::FROZEN_RIVER,
    &Biome::RIVER,
    &Biome::ICE_SPIKES,
    &Biome::OLD_GROWTH_PINE_TAIGA,
    &Biome::SUNFLOWER_PLAINS,
    &Biome::OLD_GROWTH_BIRCH_FOREST,
    &Biome::SPARSE_JUNGLE,
    &Biome::BAMBOO_JUNGLE,
    &Biome::ERODED_BADLANDS,
    &Biome::WINDSWEPT_SAVANNA,
    &Biome::CHERRY_GROVE,
    &Biome::FROZEN_PEAKS,
    &Biome::DRIPSTONE_CAVES,
    &Biome::LUSH_CAVES,
    &Biome::SULFUR_CAVES,
    &Biome::DEEP_DARK,
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct FeatureData {
    step: usize,
    encounter_index: usize,
    feature: PlacedFeature,
}

impl Ord for FeatureData {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.step, self.encounter_index, self.feature).cmp(&(
            other.step,
            other.encounter_index,
            other.feature,
        ))
    }
}

impl PartialOrd for FeatureData {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

static OVERWORLD_FEATURES_PER_STEP: LazyLock<Vec<Vec<PlacedFeature>>> =
    LazyLock::new(|| sort_features_per_step(OVERWORLD_BIOMES));
static OVERWORLD_BIOME_MASK: LazyLock<[u64; 4]> = LazyLock::new(|| {
    let mut mask = [0u64; 4];
    for biome in OVERWORLD_BIOMES {
        let id = biome.id as usize;
        mask[id / 64] |= 1u64 << (id % 64);
    }
    mask
});

#[inline(always)]
fn is_overworld_biome(id: u8) -> bool {
    let id = id as usize;
    (OVERWORLD_BIOME_MASK[id / 64] & (1u64 << (id % 64))) != 0
}

#[derive(Clone, Copy, Default)]
struct PlacedFeatureSet {
    bits: [u64; 5],
}

impl PlacedFeatureSet {
    #[inline(always)]
    fn insert(&mut self, feature: PlacedFeature) {
        let idx = feature as usize;
        self.bits[idx / 64] |= 1u64 << (idx % 64);
    }

    #[inline(always)]
    fn contains(&self, feature: PlacedFeature) -> bool {
        let idx = feature as usize;
        (self.bits[idx / 64] & (1u64 << (idx % 64))) != 0
    }
}

#[inline]
pub fn for_each_selected_feature(
    biome_ids: &[u8],
    step: usize,
    mut f: impl FnMut(usize, PlacedFeature),
) {
    if !biome_ids.iter().any(|&biome_id| is_overworld_biome(biome_id)) {
        for (index, feature) in select_features(biome_ids, step) {
            f(index, feature);
        }
        return;
    }

    let Some(features) = OVERWORLD_FEATURES_PER_STEP.get(step) else {
        return;
    };
    if features.is_empty() {
        return;
    }

    let mut selected = PlacedFeatureSet::default();

    for &biome_id in biome_ids {
        if !is_overworld_biome(biome_id) {
            continue;
        }
        if let Some(features) = Biome::from_id(biome_id).and_then(|biome| biome.features.get(step))
        {
            for &feature in *features {
                selected.insert(feature);
            }
        }
    }

    for (global_index, &feature) in features.iter().enumerate() {
        if selected.contains(feature) {
            f(global_index, feature);
        }
    }
}

pub fn select_features(biome_ids: &[u8], step: usize) -> Vec<(usize, PlacedFeature)> {
    // Feature generation does not currently carry its biome-source identity.
    // TODO: Consider modeling the Nether and End biome feature orders
    if !biome_ids.iter().any(|&biome_id| is_overworld_biome(biome_id)) {
        let mut selected: Vec<_> = biome_ids
            .iter()
            .filter_map(|biome_id| Biome::from_id(*biome_id))
            .filter_map(|biome| biome.features.get(step))
            .flat_map(|features| features.iter().copied())
            .collect();
        selected.sort_unstable();
        selected.dedup();
        return selected.into_iter().enumerate().collect();
    }

    let mut result = Vec::new();
    for_each_selected_feature(biome_ids, step, |global_index, feature| {
        result.push((global_index, feature));
    });
    result
}

fn sort_features_per_step(biomes: &[&Biome]) -> Vec<Vec<PlacedFeature>> {
    let mut feature_indices = HashMap::new();
    let mut edges: BTreeMap<FeatureData, BTreeSet<FeatureData>> = BTreeMap::new();
    let mut max_steps = 0;

    for biome in biomes {
        max_steps = max_steps.max(biome.features.len());
        let mut biome_features = Vec::new();

        for (step, features) in biome.features.iter().enumerate() {
            for &feature in *features {
                let next_index = feature_indices.len();
                let encounter_index = *feature_indices.entry(feature).or_insert(next_index);
                let data = FeatureData {
                    step,
                    encounter_index,
                    feature,
                };
                edges.entry(data).or_default();
                biome_features.push(data);
            }
        }

        for pair in biome_features.windows(2) {
            edges.entry(pair[0]).or_default().insert(pair[1]);
        }
    }

    let mut discovered = BTreeSet::new();
    let mut visiting = BTreeSet::new();
    let mut sorted = Vec::with_capacity(edges.len());
    for &feature in edges.keys() {
        visit_feature(feature, &edges, &mut discovered, &mut visiting, &mut sorted);
    }
    sorted.reverse();

    let mut per_step = vec![Vec::new(); max_steps];
    for feature in sorted {
        per_step[feature.step].push(feature.feature);
    }
    per_step
}

fn visit_feature(
    feature: FeatureData,
    edges: &BTreeMap<FeatureData, BTreeSet<FeatureData>>,
    discovered: &mut BTreeSet<FeatureData>,
    visiting: &mut BTreeSet<FeatureData>,
    sorted: &mut Vec<FeatureData>,
) {
    if discovered.contains(&feature) {
        return;
    }
    assert!(
        visiting.insert(feature),
        "feature order cycle contains {feature:?}"
    );
    if let Some(next_features) = edges.get(&feature) {
        for &next in next_features {
            visit_feature(next, edges, discovered, visiting, sorted);
        }
    }
    visiting.remove(&feature);
    discovered.insert(feature);
    sorted.push(feature);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dry_grass_uses_vanilla_global_feature_index() {
        let step = &OVERWORLD_FEATURES_PER_STEP[9];
        assert_eq!(
            step.iter()
                .position(|feature| *feature == PlacedFeature::PatchDryGrassDesert),
            Some(69)
        );
        assert_eq!(
            step.iter()
                .position(|feature| *feature == PlacedFeature::PatchDryGrassBadlands),
            Some(71)
        );
    }

    #[test]
    fn biome_selection_keeps_global_indices_and_order() {
        let selected = select_features(&[Biome::SAVANNA.id, Biome::DESERT.id], 9);
        assert!(selected.windows(2).all(|pair| pair[0].0 < pair[1].0));
        assert!(selected.contains(&(69, PlacedFeature::PatchDryGrassDesert)));
        assert!(
            !selected
                .iter()
                .any(|(_, feature)| { *feature == PlacedFeature::PatchDryGrassBadlands })
        );

        let savanna_grass = selected
            .iter()
            .find(|(_, feature)| *feature == PlacedFeature::PatchGrassSavanna)
            .expect("savanna grass must be selected");
        assert_eq!(savanna_grass.0, 12);
    }

    #[test]
    fn selection_deduplicates_biomes_from_neighbor_chunks() {
        let once = select_features(&[Biome::DESERT.id], 9);
        let repeated = select_features(&[Biome::DESERT.id, Biome::DESERT.id, Biome::DESERT.id], 9);
        assert_eq!(once, repeated);
    }

    #[test]
    fn non_overworld_selection_preserves_local_indexing() {
        let selected = select_features(&[Biome::NETHER_WASTES.id], 9);
        assert!(!selected.is_empty());
        assert!(
            selected
                .iter()
                .enumerate()
                .all(|(index, (feature_index, _))| index == *feature_index)
        );
    }

    #[test]
    fn dump_step_9_features() {
        use pumpkin_util::random::{RandomImpl, get_decorator_seed, worldgen_random::WorldgenRandom};
        let world_seed = 1789322517659391064u64;
        let cx = -20;
        let cz = -28;
        let origin_x = cx * 16;
        let origin_z = cz * 16;
        let pop_seed = WorldgenRandom::get_population_seed(world_seed, origin_x, origin_z);
        println!("Pumpkin decorationSeed: 0x{:x}", pop_seed);
        let dec_seed = get_decorator_seed(pop_seed, 7, 9);
        println!("Pumpkin decorator_seed for TreesJungle: 0x{:x}", dec_seed);
        let mut rand = WorldgenRandom::from_seed(dec_seed);
        println!("First 20 numbers from Pumpkin trees_jungle RNG:");
        for i in 0..20 {
            println!("  {i}: next_bounded_i32(16)={}, next_f32={}", rand.next_bounded_i32(16), rand.next_f32());
        }
    }
}
