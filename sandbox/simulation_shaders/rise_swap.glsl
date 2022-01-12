#version 450

#include "includes.glsl"

// Rise on another matter and swap kernel
void cellular_automata_rise_swap(ivec2 pos) {
    Matter current = read_matter(pos);
    Matter up = get_neighbor(pos, UP);
    Matter down = get_neighbor(pos, DOWN);
    Matter m = current;
    if (!is_at_border_bottom() && rises_on_swap(down, current)) {
        m = down;
    } else if (!is_at_border_top() && rises_on_swap(current, up)) {
        m = up;
    }
    write_matter(pos, m);
}

void main() {
    cellular_automata_rise_swap(get_current_sim_pos());
}