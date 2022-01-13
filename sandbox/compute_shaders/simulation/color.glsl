#version 450

#include "includes.glsl"

void write_color_to_image(ivec2 pos) {
    int index = get_index(pos);
    Matter matter = read_matter(pos);
    vec4 color;
    if (is_object(matter)) {
        color = color_i32_to_vec4(int(get_objects_color(pos)));
    } else {
        color = vary_color_rgb(color_i32_to_vec4(int(matter_colors[matter.matter])), pos);
    }
    write_image_color(pos, color);
}

void main() {
    write_color_to_image(get_current_sim_pos());
}