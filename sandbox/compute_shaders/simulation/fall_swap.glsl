#version 450

#include "includes.glsl"

// Fall on another matter and swap kernel
void cellular_automata_fall_swap(ivec2 pos) {
    Matter current = read_matter(pos);
    Matter up = get_neighbor(pos, UP);
    Matter down = get_neighbor(pos, DOWN);
    Matter m = current;
    if (!is_at_border_top() && falls_on_swap(up, current)) {
        m = up;
    } else if (!is_at_border_bottom() && falls_on_swap(current, down)) {
        m = down;
    }
    write_matter(pos, m);
}

void main() {
    cellular_automata_fall_swap(get_current_sim_pos());
}