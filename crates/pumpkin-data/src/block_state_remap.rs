#[path = "block_state_remap_26_3.rs"]
mod block_state_remap_26_3;
pub use block_state_remap_26_3::BLOCK_STATE_REMAP_26_2_TO_26_3;
use pumpkin_util::version::JavaMinecraftVersion;

#[must_use]
pub fn remap_block_state_for_version(state_id: u16, version: JavaMinecraftVersion) -> u16 {
    if version >= JavaMinecraftVersion::V_26_3 {
        if (32366..32462).contains(&state_id) {
            return 3705 + (state_id - 32366);
        }
        if (32462..33742).contains(&state_id) {
            return 2425 + (state_id - 32462);
        }
        if (33742..33758).contains(&state_id) {
            return 2286 + (state_id - 33742);
        }
        if state_id == 33758 {
            return 2367;
        }
        if (33759..33767).contains(&state_id) {
            return 11227 + (state_id - 33759);
        }
        if (33767..35143).contains(&state_id) {
            return 17027 + (state_id - 33767);
        }
        if state_id == 35143 {
            return 27;
        }
        if (35144..35146).contains(&state_id) {
            return 86 + (state_id - 35144);
        }
        if (35146..35149).contains(&state_id) {
            return 166 + (state_id - 35146);
        }
        if (35149..35152).contains(&state_id) {
            return 204 + (state_id - 35149);
        }
        if (35152..35155).contains(&state_id) {
            return 264 + (state_id - 35152);
        }
        if (35155..35158).contains(&state_id) {
            return 234 + (state_id - 35155);
        }
        if (35158..35186).contains(&state_id) {
            return 519 + (state_id - 35158);
        }
        if (35186..35214).contains(&state_id) {
            return 547 + (state_id - 35186);
        }
        if (35214..35242).contains(&state_id) {
            return 575 + (state_id - 35214);
        }
        if (35242..35306).contains(&state_id) {
            return 4732 + (state_id - 35242);
        }
        if (35306..35338).contains(&state_id) {
            return 7179 + (state_id - 35306);
        }
        if (35338..35346).contains(&state_id) {
            return 7487 + (state_id - 35338);
        }
        if (35346..35410).contains(&state_id) {
            return 8207 + (state_id - 35346);
        }
        if (35410..35418).contains(&state_id) {
            return 8407 + (state_id - 35410);
        }
        if (35418..35420).contains(&state_id) {
            return 8547 + (state_id - 35418);
        }
        if (35420..35484).contains(&state_id) {
            return 9360 + (state_id - 35420);
        }
        if state_id == 35484 {
            return 12381;
        }
        if (35485..35509).contains(&state_id) {
            return 12634 + (state_id - 35485);
        }
        if (35509..35589).contains(&state_id) {
            return 14139 + (state_id - 35509);
        }
        if (35589..35595).contains(&state_id) {
            return 15231 + (state_id - 35589);
        }
        if (35595..35627).contains(&state_id) {
            return 15587 + (state_id - 35595);
        }
        if (35627..35659).contains(&state_id) {
            return 15907 + (state_id - 35627);
        }
        if (35659..35723).contains(&state_id) {
            return 16483 + (state_id - 35659);
        }
        BLOCK_STATE_REMAP_26_2_TO_26_3
            .get(state_id as usize)
            .copied()
            .unwrap_or(state_id)
    } else {
        if state_id >= 32366 {
            return 1;
        }
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
