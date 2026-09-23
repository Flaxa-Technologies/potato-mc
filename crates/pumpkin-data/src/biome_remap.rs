use pumpkin_util::version::JavaMinecraftVersion;

#[must_use]
pub fn remap_biome_for_version(biome_id: u8, version: JavaMinecraftVersion) -> u8 {
    if version >= JavaMinecraftVersion::V_26_3 {
        biome_id
    } else if version == JavaMinecraftVersion::V_26_2 {
        // Minecraft 26.3 introduced dappled_forest at ID 8 (total 67 biomes: 0..=66).
        // For clients on 26.2, 66 biomes exist (IDs 0..=65).
        // Biomes 0..=7 are identical.
        // Biome 8 (dappled_forest) didn't exist in 26.2 -> fallback to forest (21).
        // Biomes 9..=66 are shifted down by 1 so wooded_badlands (66) maps to 65.
        match biome_id {
            0..=7 => biome_id,
            8 => 21, // Forest fallback for 26.2 clients
            9..=66 => biome_id - 1,
            _ => biome_id.min(65),
        }
    } else {
        // For clients on 26.1 and earlier, 65 biomes exist (IDs 0..=64).
        // Biomes 0..=7 are identical.
        // Biome 8 (dappled_forest) didn't exist in 26.1 -> fallback to forest (21).
        // Biomes 9..=53 are shifted down by 1 (mapping to 8..=52 in 26.1).
        // Biome 54 (sulfur_caves) didn't exist in 26.1 -> fallback to dripstone_caves (15).
        // Biomes 55..=66 are shifted down by 2 (mapping to 53..=64 in 26.1).
        match biome_id {
            0..=7 => biome_id,
            8 => 21, // Forest fallback
            9..=53 => biome_id - 1,
            54 => 15, // Dripstone Caves fallback
            55..=66 => biome_id - 2,
            _ => biome_id.min(64),
        }
    }
}

#[must_use]
pub fn remap_biome_from_v26_2_to_v26_3(biome_id: u8) -> u8 {
    if biome_id >= 8 {
        biome_id + 1
    } else {
        biome_id
    }
}
