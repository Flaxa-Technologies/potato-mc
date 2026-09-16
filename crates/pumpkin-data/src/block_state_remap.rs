#[path = "block_state_remap_26_3.rs"]
mod block_state_remap_26_3;
pub use block_state_remap_26_3::BLOCK_STATE_REMAP_26_2_TO_26_3;
use pumpkin_util::version::JavaMinecraftVersion;

#[must_use]
pub fn remap_block_state_for_version(state_id: u16, version: JavaMinecraftVersion) -> u16 {
    if version >= JavaMinecraftVersion::V_26_3 {
        BLOCK_STATE_REMAP_26_2_TO_26_3
            .get(state_id as usize)
            .copied()
            .unwrap_or(state_id)
    } else {
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
}
