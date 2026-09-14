use pumpkin_data::noise_router::WrapperType;
use pumpkin_util::math::vector3::Vector3;

use crate::{
    biome::multi_noise::{NoiseValuePoint, to_long},
    generation::biome_coords,
};

use super::{
    chunk_density_function::{Cache, ChunkSpecificNoiseFunctionComponent},
    chunk_noise_router::ChunkNoiseFunctionComponent,
    density_function::{NoiseFunctionComponentRange, PassThrough},
    density_volume::{DensityBuffer, DensityVolume},
    proto_noise_router::{
        BEARDIFIER_ZERO_CONSTANT, ProtoMultiNoiseRouter, ProtoNoiseFunctionComponent,
    },
};

pub struct MultiNoiseSampler<'a> {
    temperature: usize,
    humidity: usize,
    continentalness: usize,
    erosion: usize,
    depth: usize,
    // AKA: Weirdness
    ridges: usize,
    component_stack: Box<[ChunkNoiseFunctionComponent<'a>]>,
    pub biome_cache: [([i64; 7], Option<&'static pumpkin_data::chunk::Biome>); 512],
}

impl<'a> MultiNoiseSampler<'a> {
    #[must_use]
    pub fn fill_volume(&mut self, volume: DensityVolume) -> [DensityBuffer; 6] {
        let indices = [
            self.temperature,
            self.humidity,
            self.continentalness,
            self.erosion,
            self.depth,
            self.ridges,
        ];
        indices.map(|index| {
            let mut buffer = DensityBuffer::acquire(&volume);
            ChunkNoiseFunctionComponent::sample_volume_from_stack(
                &mut self.component_stack[..=index],
                &mut buffer,
                &volume,
            );
            buffer
        })
    }

    #[must_use]
    pub fn fill_overworld_column_cached(
        &mut self,
        col_volume: &DensityVolume,
        depth_volume: &DensityVolume,
    ) -> ([DensityBuffer; 5], DensityBuffer) {
        let indices = [
            self.temperature,
            self.humidity,
            self.continentalness,
            self.erosion,
            self.ridges,
        ];
        let col_buffers = indices.map(|index| {
            let mut buffer = DensityBuffer::acquire(col_volume);
            ChunkNoiseFunctionComponent::sample_volume_from_stack(
                &mut self.component_stack[..=index],
                &mut buffer,
                col_volume,
            );
            buffer
        });
        let mut depth_buffer = DensityBuffer::acquire(depth_volume);
        ChunkNoiseFunctionComponent::sample_volume_from_stack(
            &mut self.component_stack[..=self.depth],
            &mut depth_buffer,
            depth_volume,
        );
        (col_buffers, depth_buffer)
    }

    pub fn sample(&mut self, biome_x: i32, biome_y: i32, biome_z: i32) -> NoiseValuePoint {
        let block_x = biome_coords::to_block(biome_x);
        let block_y = biome_coords::to_block(biome_y);
        let block_z = biome_coords::to_block(biome_z);

        let pos = Vector3::new(block_x, block_y, block_z);

        let temperature = ChunkNoiseFunctionComponent::sample_from_stack(
            &mut self.component_stack[..=self.temperature],
            &pos,
        );

        let humidity = ChunkNoiseFunctionComponent::sample_from_stack(
            &mut self.component_stack[..=self.humidity],
            &pos,
        );

        let continentalness = ChunkNoiseFunctionComponent::sample_from_stack(
            &mut self.component_stack[..=self.continentalness],
            &pos,
        );

        let erosion = ChunkNoiseFunctionComponent::sample_from_stack(
            &mut self.component_stack[..=self.erosion],
            &pos,
        );

        let depth = ChunkNoiseFunctionComponent::sample_from_stack(
            &mut self.component_stack[..=self.depth],
            &pos,
        );

        let weirdness = ChunkNoiseFunctionComponent::sample_from_stack(
            &mut self.component_stack[..=self.ridges],
            &pos,
        );

        NoiseValuePoint {
            temperature: to_long(temperature),
            humidity: to_long(humidity),
            continentalness: to_long(continentalness),
            erosion: to_long(erosion),
            depth: to_long(depth),
            weirdness: to_long(weirdness),
        }
    }

    pub fn sample_erosion(&mut self, block_x: i32, block_y: i32, block_z: i32) -> f32 {
        let pos = Vector3::new(block_x, block_y, block_z);

        ChunkNoiseFunctionComponent::sample_from_stack(
            &mut self.component_stack[..=self.erosion],
            &pos,
        )
    }

    #[must_use]
    pub fn generate(base: &'a ProtoMultiNoiseRouter) -> Self {
        // TODO: It seems kind of wasteful to iter over all components (even those we dont need
        // because they're for chunk population), but this is the best I've got for now.
        // (Should we traverse the functions and update the indices?)
        let mut component_stack =
            Vec::<ChunkNoiseFunctionComponent>::with_capacity(base.full_component_stack.len());
        for base_component in &base.full_component_stack {
            let chunk_component = match base_component {
                ProtoNoiseFunctionComponent::Dependent(dependent) => {
                    ChunkNoiseFunctionComponent::Dependent(dependent)
                }
                ProtoNoiseFunctionComponent::Independent(independent) => {
                    ChunkNoiseFunctionComponent::Independent(independent)
                }
                ProtoNoiseFunctionComponent::PassThrough(pass_through) => {
                    ChunkNoiseFunctionComponent::PassThrough(pass_through.clone())
                }
                ProtoNoiseFunctionComponent::Beardifier(_) => {
                    ChunkNoiseFunctionComponent::Independent(&BEARDIFIER_ZERO_CONSTANT)
                }
                ProtoNoiseFunctionComponent::Wrapper(wrapper) => {
                    //NOTE: Due to our previous invariant with the proto-function, it is guaranteed
                    // that the wrapped function is already on the stack
                    let min_value = component_stack[wrapper.input_index].min();
                    let max_value = component_stack[wrapper.input_index].max();

                    match wrapper.wrapper_type {
                        WrapperType::Cache => ChunkNoiseFunctionComponent::Chunk(
                            ChunkSpecificNoiseFunctionComponent::Cache(Cache::new(
                                wrapper.input_index,
                                min_value,
                                max_value,
                            )),
                        ),
                        WrapperType::Interpolated { .. } => {
                            ChunkNoiseFunctionComponent::PassThrough(PassThrough::new(
                                wrapper.input_index,
                                min_value,
                                max_value,
                            ))
                        }
                    }
                }
            };
            component_stack.push(chunk_component);
        }

        Self {
            temperature: base.temperature,
            humidity: base.vegetation,
            continentalness: base.continents,
            depth: base.depth,
            erosion: base.erosion,
            ridges: base.ridges,
            component_stack: component_stack.into_boxed_slice(),
            biome_cache: [([0i64; 7], None); 512],
        }
    }
}

#[cfg(test)]
mod test {
    use pumpkin_data::noise_router::OVERWORLD_BASE_NOISE_ROUTER;

    use crate::{
        GlobalRandomConfig, biome::multi_noise::NoiseValuePoint,
        generation::noise::router::proto_noise_router::ProtoNoiseRouters,
    };

    use super::MultiNoiseSampler;

    #[test]
    fn sample() {
        let seed = 123;
        let random_config = GlobalRandomConfig::new(seed, false);
        let noise_router =
            ProtoNoiseRouters::generate(&OVERWORLD_BASE_NOISE_ROUTER, &random_config);
        let mut sampler = MultiNoiseSampler::generate(&noise_router.multi_noise);
        let expected = NoiseValuePoint {
            temperature: -5727,
            humidity: 55,
            continentalness: 4996,
            erosion: 2371,
            depth: -19774,
            weirdness: 4421,
        };
        assert_eq!(sampler.sample(123, 123, 123), expected);
    }

    #[test]
    fn sample_2() {
        // we use a different seed
        let seed = 13579;
        let random_config = GlobalRandomConfig::new(seed, false);
        let noise_router =
            ProtoNoiseRouters::generate(&OVERWORLD_BASE_NOISE_ROUTER, &random_config);
        let mut sampler = MultiNoiseSampler::generate(&noise_router.multi_noise);
        let expected = NoiseValuePoint {
            temperature: 7489,
            humidity: 3502,
            continentalness: -2168,
            erosion: -3511,
            depth: -21237,
            weirdness: -5222,
        };
        assert_eq!(sampler.sample(123, 123, 123), expected);
    }
}
