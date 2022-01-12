#version 450

#include "includes.glsl"

void cellular_automata_move_left_swap(ivec2 pos) {
    Matter current = read_matter(pos);
    Matter right = get_neighbor(pos, RIGHT);
    Matter left = get_neighbor(pos, LEFT);
    Matter right_right = get_neighbor(get_pos_at_dir(pos, RIGHT), RIGHT);

    Matter m = current;
    if (!is_at_border_right() && moves_on_swap_certainly(right, current, right_right)) {
        m = right;
    } else if (!is_at_border_left() && moves_on_swap_certainly(current, left, right)) {
        m = left;
    } else if (!is_at_border_right() && moves_on_swap_maybe(right, current, right_right,
                rand(get_pos_at_dir(pos, RIGHT), push_constants.seed))) {
        m = right;
    } else if (!is_at_border_left() && moves_on_swap_maybe(current, left, right, rand(pos, push_constants.seed))) {
        m = left;
    }
    write_matter(pos, m);
}

void cellular_automata_move_right_swap(ivec2 pos) {
    Matter current = read_matter(pos);
    Matter right = get_neighbor(pos, RIGHT);
    Matter left = get_neighbor(pos, LEFT);
    Matter left_left = get_neighbor(get_pos_at_dir(pos, LEFT), LEFT);

    Matter m = current;
    if (!is_at_border_left() && moves_on_swap_certainly(left, current, left_left)) {
        m = left;
    } else if (!is_at_border_right() && moves_on_swap_certainly(current, right, left)) {
        m = right;
    } else if (!is_at_border_left() && moves_on_swap_maybe(left, current, left_left,
                rand(get_pos_at_dir(pos, LEFT), push_constants.seed))) {
        m = left;
    } else if (!is_at_border_right() && moves_on_swap_maybe(current, right, left, rand(pos, push_constants.seed))) {
        m = right;
    }
    write_matter(pos, m);
}

void cellular_automata_move_horizontal_swap(ivec2 pos) {
    if (push_constants.dispersion_dir == 0) {
        cellular_automata_move_left_swap(pos);
    } else {
        cellular_automata_move_right_swap(pos);
    }
}

void main() {
    cellular_automata_move_horizontal_swap(get_current_sim_pos());
}