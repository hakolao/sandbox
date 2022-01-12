#version 450

#include "includes.glsl"

void init(ivec2 pos) {
    Matter matter = read_matter(pos);
    // Save matter underneath
    if (is_object(matter)) {
        tmp_matter[get_index(ivec2(gl_GlobalInvocationID.xy))] = get_matter_in(pos);
    }
    write_matter(pos,  read_matter(pos));
}

void main() {
    init(get_current_sim_pos());
}