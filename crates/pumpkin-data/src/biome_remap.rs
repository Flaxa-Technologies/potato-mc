use pumpkin_util::version::JavaMinecraftVersion;

#[must_use]
pub fn remap_biome_for_version(biome_id: u8, version: JavaMinecraftVersion) -> u8 {
    if version >= JavaMinecraftVersion::V_26_2 {
        biome_id
    } else {
        // Minecraft 26.2 introduced sulfur_caves at ID 53 (total 66 biomes: 0..=65).
        // For clients on 26.1 and earlier, only 65 biomes exist (IDs 0..=64).
        // Biomes 0..52 are identical across 26.1 and 26.2.
        // Biome 53 (sulfur_caves) didn't exist in 26.1 -> fallback to dripstone_caves (15).
        // Biomes 54..=65 are shifted down by 1 so wooded_badlands (65) maps to 64.
        match biome_id {
            0..=52 => biome_id,
            53 => 15, // Dripstone Caves fallback for pre-26.2 clients
            54..=65 => biome_id - 1,
            _ => biome_id.min(64),
        }
    }
}
