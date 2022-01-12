#version 450

#include "includes.glsl"

void finish(ivec2 pos) {
    int index = get_index(pos);
    Matter matter = read_matter(pos);
    vec4 color;
    if (is_object(matter)) {
        // Get color from object
        color = color_i32_to_vec4(int(get_objects_color(pos)));
        // Get matter underneath
        int local_index = get_index(ivec2(gl_GlobalInvocationID.xy));
        uint matter_underneath = tmp_matter[local_index];
        // Place back matter undeneath
        matter = new_matter(matter_underneath);
    } else {
        // ToDo: Handle color variation with a separate shader
        color = vary_color_rgb(color_i32_to_vec4(int(matter_colors[matter.matter])), pos);
    }
    write_image_color(pos, color);
    // We make an exception here to write to both matter in & out to ensure correct grid state
    // This might affect the result of "update_bitmap"
    write_matter_both(pos, matter);
}

void main() {
    finish(get_current_sim_pos());
    // Clear tmp grid in update_bitmaps
}