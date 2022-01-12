#version 450

#include "includes.glsl"

// Also matches zero == zero
bool any_bit_set_and_zero(uint a, uint b) {
    return (a & b) != 0 || (a == b);
}

bool is_bit_set(uint a, int bit_location) {
    return (a & (uint(1) << bit_location)) != 0;
}

bool interacts_with_reactive(uint reacts, uint reacts_direction,
Matter up, Matter down, Matter left, Matter right,
Matter up_left, Matter up_right, Matter down_left, Matter down_right) {
    bool reacts_up_left = false;
    bool reacts_up = false;
    bool reacts_up_right = false;
    bool reacts_right = false;
    bool reacts_down_right = false;
    bool reacts_down = false;
    bool reacts_down_left = false;
    bool reacts_left = false;

    if (is_bit_set(reacts_direction, UP_LEFT)) {
        reacts_up_left = any_bit_set_and_zero(up_left.characteristics, reacts);
    }
    if (is_bit_set(reacts_direction, UP)) {
        reacts_up = any_bit_set_and_zero(up.characteristics, reacts);
    }
    if (is_bit_set(reacts_direction, UP_RIGHT)) {
        reacts_up_right = any_bit_set_and_zero(up_right.characteristics, reacts);
    }
    if (is_bit_set(reacts_direction, RIGHT)) {
        reacts_right = any_bit_set_and_zero(right.characteristics, reacts);
    }
    if (is_bit_set(reacts_direction, DOWN_RIGHT)) {
        reacts_down_right = any_bit_set_and_zero(down_right.characteristics, reacts);
    }
    if (is_bit_set(reacts_direction, DOWN)) {
        reacts_down = any_bit_set_and_zero(down.characteristics, reacts);
    }
    if (is_bit_set(reacts_direction, DOWN_LEFT)) {
        reacts_down_left = any_bit_set_and_zero(down_left.characteristics, reacts);
    }
    if (is_bit_set(reacts_direction, LEFT)) {
        reacts_left = any_bit_set_and_zero(left.characteristics, reacts);
    }
    return reacts_up_left || reacts_up || reacts_up_right || reacts_right || reacts_down_right || reacts_down || reacts_down_left ||  reacts_left;
}

bool transition_occurs(uint reacts, uint reacts_direction, float p, float transition_probability,
Matter up, Matter down, Matter left, Matter right,
Matter up_left, Matter up_right, Matter down_left, Matter down_right) {
    return p < transition_probability &&
    interacts_with_reactive(reacts, reacts_direction, up, down, left, right, up_left, up_right, down_left, down_right);
}

// A matter will transition into another matter if it reacts with neighbors (touches / collides whatever)
Matter transition_into(Matter current, ivec2 pos) {
    Matter up = get_neighbor(pos, UP);
    Matter down = get_neighbor(pos, DOWN);
    Matter left = get_neighbor(pos, LEFT);
    Matter right = get_neighbor(pos, RIGHT);

    Matter up_left = get_neighbor(pos, UP_LEFT);
    Matter up_right = get_neighbor(pos, UP_RIGHT);
    Matter down_left = get_neighbor(pos, DOWN_LEFT);
    Matter down_right = get_neighbor(pos, DOWN_RIGHT);

    Matter m = current;

    float p = rand(pos, push_constants.seed);
    uint reacts = current.reacts[0];
    uint reacts_direction = current.reacts_direction[0];
    float reaction_probability = current.reaction_probability[0];
    uint reaction_transition = current.reaction_transition[0];
    if (transition_occurs(reacts, reacts_direction, p,
    reaction_probability, up, down, left, right, up_left, up_right, down_left, down_right)) {
        m = new_matter(reaction_transition);
        return m;
    }

    p = rand(pos, push_constants.seed + 1.0);
    reacts = current.reacts[1];
    reaction_probability = current.reaction_probability[1];
    reacts_direction = current.reacts_direction[1];
    reaction_transition = current.reaction_transition[1];
    if (transition_occurs(reacts, reacts_direction, p,
    reaction_probability, up, down, left, right, up_left, up_right, down_left, down_right)) {
        m = new_matter(reaction_transition);
        return m;
    }

    p = rand(pos, push_constants.seed + 2.0);
    reacts = current.reacts[2];
    reaction_probability = current.reaction_probability[2];
    reacts_direction = current.reacts_direction[2];
    reaction_transition = current.reaction_transition[2];
    if (transition_occurs(reacts, reacts_direction, p,
    reaction_probability, up, down, left, right, up_left, up_right, down_left, down_right)) {
        m = new_matter(reaction_transition);
        return m;
    }

    p = rand(pos, push_constants.seed + 3.0);
    reacts = current.reacts[3];
    reaction_probability = current.reaction_probability[3];
    reacts_direction = current.reacts_direction[3];
    reaction_transition = current.reaction_transition[3];
    if (transition_occurs(reacts, reacts_direction, p,
    reaction_probability, up, down, left, right, up_left, up_right, down_left, down_right)) {
        m = new_matter(reaction_transition);
        return m;
    }

    p = rand(pos, push_constants.seed + 4.0);
    reacts = current.reacts[4];
    reaction_probability = current.reaction_probability[4];
    reacts_direction = current.reacts_direction[4];
    reaction_transition = current.reaction_transition[4];
    if (transition_occurs(reacts, reacts_direction, p,
    reaction_probability, up, down, left, right, up_left, up_right, down_left, down_right)) {
        m = new_matter(reaction_transition);
        return m;
    }
    return m;
}

void cellular_automata_react(ivec2 pos) {
    Matter current = read_matter(pos);
    Matter m = transition_into(current, pos);
    // If object e.g. caught fire, its pixel should no longer exist in the object grid...
    if (m.matter != current.matter && is_object(current)) {
        write_objects_matter(pos, empty);
    }
    write_matter(pos, m);
}

void main() {
    cellular_automata_react(get_current_sim_pos());
}