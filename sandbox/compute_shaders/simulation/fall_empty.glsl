#version 450

#include "includes.glsl"

void cellular_automata_fall_empty(ivec2 pos) {
    Matter current = read_matter(pos);
    Matter up = get_neighbor(pos, UP);
    Matter down = get_neighbor(pos, DOWN);
    Matter m = current;
    if (!is_at_border_top() && falls_on_empty(up, current)) {
        m = up;
    } else if (!is_at_border_bottom() && falls_on_empty(current, down)) {
        m = down;
    }
    write_matter(pos, m);
}

void main() {
    cellular_automata_fall_empty(get_current_sim_pos());
}