use pumpkin_util::random::{get_decorator_seed, worldgen_random::WorldgenRandom, RandomImpl};

#[test]
fn test_find_all_ore_dirt() {
    let world_seed = 1789322517659391064i64;
    for cx in -10..=-8 {
        for cz in -22..=-19 {
            let start_x = cx * 16;
            let start_z = cz * 16;
            let pop_seed = WorldgenRandom::get_population_seed(world_seed as u64, start_x, start_z);
            let dec_seed = get_decorator_seed(pop_seed, 0, 6);
            let mut rand = WorldgenRandom::from_seed(dec_seed);
            
            for count in 0..7 {
                let lx = rand.next_bounded_i32(16);
                let lz = rand.next_bounded_i32(16);
                let y = rand.next_inbetween_i32(0, 160);
                let gx = start_x + lx;
                let gz = start_z + lz;
                let dir = rand.next_f32() * std::f32::consts::PI;
                let y0_off = rand.next_bounded_i32(3) - 2;
                let y1_off = rand.next_bounded_i32(3) - 2;
                
                // Check if gx is near -137 and gz is near -319
                if (gx - (-137)).abs() <= 16 && (gz - (-319)).abs() <= 16 {
                    println!(
                        "FOUND ORE_DIRT in chunk ({}, {}), count {}: gx={}, gz={}, y={}, dir={}, y0_off={}, y1_off={}",
                        cx, cz, count, gx, gz, y, dir, y0_off, y1_off
                    );
                }
            }
        }
    }
}
