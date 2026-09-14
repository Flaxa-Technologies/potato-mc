use pumpkin_util::random::{get_decorator_seed, worldgen_random::WorldgenRandom, RandomImpl};

#[test]
fn test_ore_dirt_rng() {
    let world_seed = 1789322517659391064i64;
    let cx = -9;
    let cz = -21;
    let start_block_x = cx * 16;
    let start_block_z = cz * 16;
    let pop_seed = WorldgenRandom::get_population_seed(world_seed as u64, start_block_x, start_block_z);
    
    // In step 6, OreDirt is global_index 0
    let dec_seed = get_decorator_seed(pop_seed, 0, 6);
    let mut rand = WorldgenRandom::from_seed(dec_seed);
    
    println!("pop_seed: 0x{:016x}", pop_seed);
    println!("dec_seed: 0x{:016x}", dec_seed);
    
    for i in 0..7 {
        let x = rand.next_bounded_i32(16);
        let z = rand.next_bounded_i32(16);
        let y = rand.next_inbetween_i32(0, 160);
        let dir = rand.next_f32() * std::f32::consts::PI;
        let y0_offset = rand.next_bounded_i32(3) - 2;
        let y1_offset = rand.next_bounded_i32(3) - 2;
        println!("Count {}: x={}, z={}, y={}, dir={}, y0_off={}, y1_off={}", i, x, z, y, dir, y0_offset, y1_offset);
    }
}
