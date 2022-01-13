#version 450

#include "includes.glsl"

void reset_bitmap(ivec2 pos) {
    ivec2 bitmap_pos = ivec2(gl_GlobalInvocationID.xy) / bitmap_ratio;
    int bitmap_size = sim_canvas_size / bitmap_ratio;
    int bitmap_index = bitmap_pos.y * bitmap_size + bitmap_pos.x;
    bitmap[bitmap_index] = 0;
}

void save_object_matter_to_tmp(ivec2 pos) {
    Matter matter = read_matter(pos);
    if (is_object(matter)) {
        tmp_matter[get_index(ivec2(gl_GlobalInvocationID.xy))] = get_matter_in(pos);
    }
}

void main() {
    reset_bitmap(get_current_sim_pos());
    save_object_matter_to_tmp(get_current_sim_pos());
}