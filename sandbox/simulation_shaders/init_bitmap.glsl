#version 450

#include "bitmap_includes.glsl"

void reset_bitmap(ivec2 pos) {
    ivec2 bitmap_pos = ivec2(gl_GlobalInvocationID.xy) / bitmap_ratio;
    int bitmap_size = sim_canvas_size / bitmap_ratio;
    int bitmap_index = bitmap_pos.y * bitmap_size + bitmap_pos.x;
    bitmap[bitmap_index] = 0;
}

void main() {
    reset_bitmap(get_current_sim_pos());
}