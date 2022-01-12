#version 450

#include "includes.glsl"

// Objects are ignored here
void update_bitmap(ivec2 pos, Matter matter) {
    if (!is_empty(matter) && current_same_as_neighbors_ignore_objects(pos, matter) &&
    (is_solid(matter) || is_powder(matter) || is_liquid(matter))) {
        int bitmap_size = sim_canvas_size / bitmap_ratio;
        ivec2 bitmap_pos = ivec2(gl_GlobalInvocationID.xy) / bitmap_ratio;
        int bitmap_index = bitmap_pos.y * bitmap_size + bitmap_pos.x;
        int solid = int(is_solid(matter));
        int powder = int(is_powder(matter));
        int liquid = int(is_liquid(matter));
        bitmap[bitmap_index] = (solid << 0) | (powder << 1) | (liquid << 2);
    }
}

void main() {
    ivec2 pos = get_current_sim_pos();
    Matter matter = new_matter(get_matter_in(pos));
    update_bitmap(pos, matter);
}