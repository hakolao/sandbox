#version 450

#include "includes.glsl"

void finish(ivec2 pos) {
    int index = get_index(pos);
    ivec2 local_pos = ivec2(gl_GlobalInvocationID.xy);
    int local_index = get_index(local_pos);
    Matter matter = read_matter(pos);
    if (is_object(matter)) {
        // Get matter underneath
        uint matter_underneath = tmp_matter[local_index];
        // Place back matter undeneath
        matter = new_matter(matter_underneath);
    }
    // We make an exception here to write to both matter in & out to ensure correct grid state
    write_matter_both(pos, matter);
    // Clear tmp grid
    tmp_matter[local_index] = empty;
}

void main() {
    finish(get_current_sim_pos());
}