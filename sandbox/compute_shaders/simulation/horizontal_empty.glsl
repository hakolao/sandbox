#version 450

#include "includes.glsl"

void cellular_automata_move_left_empty(ivec2 pos) {
    Matter current = read_matter(pos);
    Matter down = get_neighbor(pos, DOWN);
    Matter right = get_neighbor(pos, RIGHT);
    Matter left = get_neighbor(pos, LEFT);
    Matter down_right = get_neighbor(pos, DOWN_RIGHT);
    Matter right_right = get_neighbor(get_pos_at_dir(pos, RIGHT), RIGHT);

    Matter m = current;
    if (!is_at_border_right() && moves_on_empty_certainly(right, current, right_right, down_right)) {
        m = right;
    } else if (!is_at_border_left() && moves_on_empty_certainly(current, left, right, down)) {
        m = left;
    } else if (!is_at_border_right() && moves_on_empty_maybe(right, current, right_right, down_right,
            rand(get_pos_at_dir(pos, RIGHT), push_constants.seed))) {
        m = right;
    } else if (!is_at_border_left() && moves_on_empty_maybe(current, left, right, down, rand(pos, push_constants.seed))) {
        m = left;
    }
    write_matter(pos, m);
}

void cellular_automata_move_right_empty(ivec2 pos) {
    Matter current = read_matter(pos);
    Matter down = get_neighbor(pos, DOWN);
    Matter right = get_neighbor(pos, RIGHT);
    Matter left = get_neighbor(pos, LEFT);
    Matter down_left = get_neighbor(pos, DOWN_LEFT);
    Matter left_left = get_neighbor(get_pos_at_dir(pos, LEFT), LEFT);

    Matter m = current;
    if (!is_at_border_left() && moves_on_empty_certainly(left, current, left_left, down_left)) {
        m = left;
    } else if (!is_at_border_right() && moves_on_empty_certainly(current, right, left, down)) {
        m = right;
    } else if (!is_at_border_left() && moves_on_empty_maybe(left, current, left_left, down_left,
            rand(get_pos_at_dir(pos, LEFT), push_constants.seed))) {
        m = left;
    } else if (!is_at_border_right() && moves_on_empty_maybe(current, right, left, down, rand(pos, push_constants.seed))) {
        m = right;
    }
    write_matter(pos, m);
}

void cellular_automata_move_horizontal_empty(ivec2 pos) {
    if (push_constants.dispersion_dir == 0) {
        cellular_automata_move_left_empty(pos);
    } else {
        cellular_automata_move_right_empty(pos);
    }
}

void main() {
    cellular_automata_move_horizontal_empty(get_current_sim_pos());
}