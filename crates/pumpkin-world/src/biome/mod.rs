use sha2::{Digest, Sha256};

use pumpkin_data::chunk::{Biome, BiomeTree, NETHER_BIOME_SOURCE, OVERWORLD_BIOME_SOURCE};

use crate::generation::noise::router::multi_noise_sampler::MultiNoiseSampler;
pub mod end;
pub mod multi_noise;
pub mod position_finder;

pub use position_finder::{
    Climate, ClimateSampler, DistanceMetric, FittestPositionFinder, FittestPositionFinderResult,
    ParameterList, RTree, RTreeLeaf, RTreeNode, RTreeSubTree, SpawnFinder, SpawnFinderResult,
};
pub use pumpkin_data::chunk::{
    Parameter, ParameterPoint, ParameterRange, TargetPoint, quantize_coord, unquantize_coord,
};


pub trait BiomeSupplier {
    fn biome(&self, x: i32, y: i32, z: i32, noise: &mut MultiNoiseSampler<'_>) -> &'static Biome;
    #[inline]
    fn biome_from_point(&self, point_list: [i64; 7], noise: &mut MultiNoiseSampler<'_>) -> &'static Biome {
        let _ = (point_list, noise);
        &Biome::PLAINS
    }
}

pub struct MultiNoiseBiomeSupplier {
    source: &'static BiomeTree,
}

impl MultiNoiseBiomeSupplier {
    pub const OVERWORLD: Self = Self::new(&OVERWORLD_BIOME_SOURCE);
    pub const NETHER: Self = Self::new(&NETHER_BIOME_SOURCE);

    const fn new(source: &'static BiomeTree) -> Self {
        Self { source }
    }

    #[inline]
    pub fn get_biome_from_point(&self, point_list: [i64; 7], noise: &mut MultiNoiseSampler<'_>) -> &'static Biome {
        let mut h = (point_list[0] as u64).wrapping_mul(0x517cc1b727220a95);
        h = (h ^ (point_list[1] as u64)).wrapping_mul(0x517cc1b727220a95);
        h = (h ^ (point_list[2] as u64)).wrapping_mul(0x517cc1b727220a95);
        h = (h ^ (point_list[3] as u64)).wrapping_mul(0x517cc1b727220a95);
        h = (h ^ (point_list[4] as u64)).wrapping_mul(0x517cc1b727220a95);
        h = (h ^ (point_list[5] as u64)).wrapping_mul(0x517cc1b727220a95);
        let idx = (h as usize) & (noise.biome_cache.len() - 1);
        if noise.biome_cache[idx].0 == point_list
            && let Some(biome) = noise.biome_cache[idx].1
        {
            return biome;
        }
        let biome = self.source.get(&point_list, &mut None);
        noise.biome_cache[idx] = (point_list, Some(biome));
        biome
    }
}

impl BiomeSupplier for MultiNoiseBiomeSupplier {
    #[inline]
    fn biome(&self, x: i32, y: i32, z: i32, noise: &mut MultiNoiseSampler<'_>) -> &'static Biome {
        let point = noise.sample(x, y, z);
        self.get_biome_from_point(point.convert_to_list(), noise)
    }

    #[inline]
    fn biome_from_point(&self, point_list: [i64; 7], noise: &mut MultiNoiseSampler<'_>) -> &'static Biome {
        self.get_biome_from_point(point_list, noise)
    }
}

#[must_use]
pub fn hash_seed(seed: u64) -> i64 {
    let mut hasher = Sha256::new();
    hasher.update(seed.to_le_bytes());
    let result = hasher.finalize();
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&result[..8]);
    i64::from_le_bytes(bytes)
}

#[cfg(test)]
mod test {
    use pumpkin_data::{chunk::Biome, dimension::Dimension};
    use pumpkin_util::read_data_from_file;
    use serde::Deserialize;

    use crate::{
        ProtoChunk, chunk::palette::BIOME_NETWORK_MAX_BITS,
        generation::noise::router::multi_noise_sampler::MultiNoiseSampler,
    };

    use super::{BiomeSupplier, MultiNoiseBiomeSupplier, hash_seed};

    #[test]
    fn biome_desert() {
        use crate::generation::generator::{GeneratorInit, VanillaGenerator};
        use pumpkin_util::world_seed::Seed;
        let seed = 13579;
        let generator = VanillaGenerator::new(Seed(seed as u64), Dimension::OVERWORLD);
        let mut sampler = MultiNoiseSampler::generate(&generator.base_router.multi_noise);
        let biome = MultiNoiseBiomeSupplier::OVERWORLD.biome(-24, 1, 8, &mut sampler);
        assert_eq!(biome, &Biome::DESERT);
    }

    #[test]
    fn wide_area_surface() {
        use crate::generation::generator::{GeneratorInit, VanillaGenerator, WorldGenerator};
        use crate::generation::noise::router::multi_noise_sampler::MultiNoiseSampler;
        use pumpkin_util::world_seed::Seed;
        #[derive(Deserialize)]
        struct BiomeData {
            x: i32,
            z: i32,
            data: Vec<(i32, i32, i32, u8)>,
        }

        let expected_data: Vec<BiomeData> =
            read_data_from_file!("../../../../assets/tests/biome_no_blend_no_beard_0.json");

        let seed = 0;
        let world_gen = WorldGenerator::Noise(Box::new(VanillaGenerator::new(
            Seed(seed as u64),
            Dimension::OVERWORLD,
        )));
        let WorldGenerator::Noise(generator) = &world_gen else {
            unreachable!()
        };

        for data in expected_data {
            let chunk_x = data.x;
            let chunk_z = data.z;

            let mut chunk = ProtoChunk::new(chunk_x, chunk_z, &world_gen);

            let mut multi_noise_sampler =
                MultiNoiseSampler::generate(&generator.base_router.multi_noise);

            chunk.populate_biomes(generator, &mut multi_noise_sampler);

            for (biome_x, biome_y, biome_z, biome_id) in data.data {
                let calculated_biome = chunk.get_biome(biome_x, biome_y, biome_z);

                let expected_id =
                    pumpkin_data::biome_remap::remap_biome_from_v26_2_to_v26_3(biome_id);
                assert_eq!(
                    expected_id,
                    calculated_biome.id,
                    "Expected {:?} was {:?} at {},{},{} ({},{})",
                    Biome::from_id(expected_id),
                    calculated_biome,
                    biome_x,
                    biome_y,
                    biome_z,
                    data.x,
                    data.z
                );
            }
        }
    }

    #[test]
    fn hash_seed_test() {
        let hashed_seed = hash_seed(0);
        assert_eq!(8794265229978523055, hashed_seed);

        let hashed_seed = hash_seed((-777i64) as u64);
        assert_eq!(-1087248400229165450, hashed_seed);
    }

    #[test]
    fn proper_network_bits_per_entry() {
        let id_to_test = 1 << BIOME_NETWORK_MAX_BITS;
        assert!(
            Biome::from_id(id_to_test).is_none(),
            "We need to update our constants!"
        );
    }
}
