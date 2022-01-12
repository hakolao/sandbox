#version 450

#include "includes.glsl"

// Slide down left on another matter and swap kernel
void cellular_automata_slide_left_swap(ivec2 pos) {
    Matter current = read_matter(pos);
    Matter down = get_neighbor(pos, DOWN);
    Matter right = get_neighbor(pos, RIGHT);
    Matter up_right = get_neighbor(pos, UP_RIGHT);
    Matter down_left = get_neighbor(pos, DOWN_LEFT);

    Matter m = current;
    if (!is_at_border_top() && !is_at_border_right() && slides_on_swap(up_right, current, right)) {
        m = up_right;
    } else if (!is_at_border_bottom() && !is_at_border_left() && slides_on_swap(current, down_left, down)) {
        m = down_left;
    }
    write_matter(pos, m);
}

// Slide down right on another matter and swap kernel
void cellular_automata_slide_right_swap(ivec2 pos) {
    Matter current = read_matter(pos);
    Matter down = get_neighbor(pos, DOWN);
    Matter left = get_neighbor(pos, LEFT);
    Matter up_left = get_neighbor(pos, UP_LEFT);
    Matter down_right = get_neighbor(pos, DOWN_RIGHT);

    Matter m = current;
    if (!is_at_border_top() && !is_at_border_left() && slides_on_swap(up_left, current, left)) {
        m = up_left;
    } else if (!is_at_border_bottom() && !is_at_border_right() && slides_on_swap(current, down_right, down)) {
        m = down_right;
    }
    write_matter(pos, m);
}

void cellular_automata_slide_down_swap(ivec2 pos) {
    if ((push_constants.sim_step + push_constants.move_step) % 2 == 0) {
        cellular_automata_slide_left_swap(pos);
    } else {
        cellular_automata_slide_right_swap(pos);
    }
}

void main() {
    cellular_automata_slide_down_swap(get_current_sim_pos());
}