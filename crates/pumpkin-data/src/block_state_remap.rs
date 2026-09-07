use pumpkin_util::version::JavaMinecraftVersion;

#[must_use]
pub fn remap_block_state_for_version(state_id: u16, version: JavaMinecraftVersion) -> u16 {
    let remapped =
        crate::block_state_remap_generated::remap_block_state_for_version(state_id, version);
    if state_id != 0 && remapped == 0 {
        // Fallback to stone (block state ID 1) for older versions (e.g. 1.21 connecting to a 26.2 server)
        // so players see a solid visible block texture instead of invisible air or being kicked.
        1
    } else {
        remapped
    }
}
