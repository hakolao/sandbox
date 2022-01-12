// Specialization constants
layout(constant_id = 0) const uint empty = 1;
layout(constant_id = 1) const int sim_canvas_size = 1;
layout(constant_id = 2) const int bitmap_ratio = 1;
layout(constant_id = 3) const uint state_empty = 1;
layout(constant_id = 4) const uint state_powder = 1;
layout(constant_id = 5) const uint state_liquid = 1;
layout(constant_id = 6) const uint state_solid = 1;
layout(constant_id = 7) const uint state_solid_gravity = 1;
layout(constant_id = 8) const uint state_gas = 1;
layout(constant_id = 9) const uint state_energy = 1;
layout(constant_id = 10) const uint state_object = 1;

// X & Y input as specialization constant
layout(local_size_x_id = 11, local_size_y_id = 12, local_size_z = 1) in;

layout(set = 0, binding = 0) buffer MatterColorsBuffer {
    uint matter_colors[];
};
layout(set = 0, binding = 1) buffer MatterStateBuffer {
    uint matter_state[];
};
layout(set = 0, binding = 2) buffer MatterWeightsBuffer {
    float matter_weights[];
};
layout(set = 0, binding = 3) buffer MatterDispersionBuffer {
    uint matter_dispersion[];
};
layout(set = 0, binding = 4) buffer MatterCharacteristicsBuffer {
    uint matter_characteristics[];
};
layout(set = 0, binding = 5) buffer MatterReactionWithBuffer {
    uint matter_reaction_with[];
};
layout(set = 0, binding = 6) buffer MatterReactionDirectionBuffer {
    uint matter_reaction_direction[];
};
layout(set = 0, binding = 7) buffer MatterReactionProbabilityBuffer {
    float matter_reaction_probability[];
};
layout(set = 0, binding = 8) buffer MatterReactionTransitionBuffer {
    uint matter_reaction_transition[];
};

/*
Matter data chunks
*/
layout(set = 0, binding = 9) buffer MatterInBuffer0 { uint matter_in0[]; };
layout(set = 0, binding = 10) writeonly buffer MatterOutBuffer0 { uint matter_out0[]; };
layout(set = 0, binding = 11) buffer ObjectsMatter0 { uint objects_matter0[]; };
layout(set = 0, binding = 12) buffer ObjectsColor0 { uint objects_color0[]; };
layout(set = 0, binding = 13, rgba8) uniform writeonly image2D canvas_img0;

layout(set = 0, binding = 14) buffer MatterInBuffer1 { uint matter_in1[]; };
layout(set = 0, binding = 15) writeonly buffer MatterOutBuffer1 { uint matter_out1[]; };
layout(set = 0, binding = 16) buffer ObjectsMatter1 { uint objects_matter1[]; };
layout(set = 0, binding = 17) buffer ObjectsColor1 { uint objects_color1[]; };
layout(set = 0, binding = 18, rgba8) uniform writeonly image2D canvas_img1;

layout(set = 0, binding = 19) buffer MatterInBuffer2 { uint matter_in2[]; };
layout(set = 0, binding = 20) writeonly buffer MatterOutBuffer2 { uint matter_out2[]; };
layout(set = 0, binding = 21) buffer ObjectsMatter2 { uint objects_matter2[]; };
layout(set = 0, binding = 22) buffer ObjectsColor2 { uint objects_color2[]; };
layout(set = 0, binding = 23, rgba8) uniform writeonly image2D canvas_img2;

layout(set = 0, binding = 24) buffer MatterInBuffer3 { uint matter_in3[]; };
layout(set = 0, binding = 25) writeonly buffer MatterOutBuffer3 { uint matter_out3[]; };
layout(set = 0, binding = 26) buffer ObjectsMatter3 { uint objects_matter3[]; };
layout(set = 0, binding = 27) buffer ObjectsColor3 { uint objects_color3[]; };
layout(set = 0, binding = 28, rgba8) uniform writeonly image2D canvas_img3;

layout(push_constant) uniform PushConstants {
    float seed;
    uint sim_step;
    uint move_step;
    uint dispersion_step;
    uint dispersion_dir;
    ivec2 sim_pos_offset;
    ivec2 sim_chunk_start_offset;
} push_constants;

#include "dirs.glsl"

#define MAX_TRANSITIONS 5

const ivec2 HALF_CANVAS = ivec2(sim_canvas_size / 2);

struct Matter {
    uint matter;
    uint state;
    uint dispersion;
    float weight;
    uint characteristics;
    uint[MAX_TRANSITIONS] reacts;
    uint[MAX_TRANSITIONS] reacts_direction;
    float[MAX_TRANSITIONS] reaction_probability;
    uint[MAX_TRANSITIONS] reaction_transition;
};

Matter new_matter(uint matter) {
    Matter m;
    m.matter = matter;
    m.state = matter_state[m.matter];
    m.weight = matter_weights[m.matter];
    m.dispersion = matter_dispersion[m.matter];
    m.characteristics = matter_characteristics[m.matter];
    uint table_index = m.matter * MAX_TRANSITIONS;
    m.reacts[0] = matter_reaction_with[table_index + 0];
    m.reacts[1] = matter_reaction_with[table_index + 1];
    m.reacts[2] = matter_reaction_with[table_index + 2];
    m.reacts[3] = matter_reaction_with[table_index + 3];
    m.reacts[4] = matter_reaction_with[table_index + 4];

    m.reacts_direction[0] = matter_reaction_direction[table_index + 0];
    m.reacts_direction[1] = matter_reaction_direction[table_index + 1];
    m.reacts_direction[2] = matter_reaction_direction[table_index + 2];
    m.reacts_direction[3] = matter_reaction_direction[table_index + 3];
    m.reacts_direction[4] = matter_reaction_direction[table_index + 4];

    m.reaction_probability[0] = matter_reaction_probability[table_index + 0];
    m.reaction_probability[1] = matter_reaction_probability[table_index + 1];
    m.reaction_probability[2] = matter_reaction_probability[table_index + 2];
    m.reaction_probability[3] = matter_reaction_probability[table_index + 3];
    m.reaction_probability[4] = matter_reaction_probability[table_index + 4];

    m.reaction_transition[0] = matter_reaction_transition[table_index + 0];
    m.reaction_transition[1] = matter_reaction_transition[table_index + 1];
    m.reaction_transition[2] = matter_reaction_transition[table_index + 2];
    m.reaction_transition[3] = matter_reaction_transition[table_index + 3];
    m.reaction_transition[4] = matter_reaction_transition[table_index + 4];
    return m;
}

// https://stackoverflow.com/questions/4200224/random-noise-functions-for-glsl
float PHI = 1.61803398874989484820459; // Golden ratio
float rand(in vec2 xy, in float seed){
    vec2 pos = vec2(xy.x + 0.5, xy.y + 0.5);
    return fract(tan(distance(pos * PHI, pos) * seed) * pos.x);
}

ivec2 get_current_sim_pos() {
    return ivec2(gl_GlobalInvocationID.xy) - HALF_CANVAS + push_constants.sim_pos_offset;
}

ivec2 get_local_pos(ivec2 pos) {
    return pos + HALF_CANVAS - push_constants.sim_pos_offset;
}

int get_index(ivec2 pos) {
    return pos.y * sim_canvas_size + pos.x;
}

ivec2 get_pos_inside_chunk(ivec2 pos) {
    ivec2 diff = pos - push_constants.sim_chunk_start_offset;
    return ivec2(diff.x % sim_canvas_size, diff.y % sim_canvas_size);
}

int get_chunk_index(ivec2 pos) {
    ivec2 pos_on_4_chunks = (pos - push_constants.sim_chunk_start_offset) / sim_canvas_size;
    return pos_on_4_chunks.y * 2 + pos_on_4_chunks.x;
}

bool is_at_border_top() {
    ivec2 local_pos = ivec2(gl_GlobalInvocationID.xy);
    return local_pos.y == sim_canvas_size - 1;
}

bool is_at_border_bottom() {
    ivec2 local_pos = ivec2(gl_GlobalInvocationID.xy);
    return local_pos.y == 0;
}

bool is_at_border_right() {
    ivec2 local_pos = ivec2(gl_GlobalInvocationID.xy);
    return local_pos.x == sim_canvas_size - 1;
}

bool is_at_border_left() {
    ivec2 local_pos = ivec2(gl_GlobalInvocationID.xy);
    return local_pos.x == 0;
}

uint get_matter_in(ivec2 pos) {
    int index = get_index(get_pos_inside_chunk(pos));
    int chunk_index = get_chunk_index(pos);
    if (chunk_index == 0) {
        return matter_in0[index];
    } else if (chunk_index == 1) {
        return matter_in1[index];
    } else if (chunk_index == 2) {
        return matter_in2[index];
    } else if (chunk_index == 3) {
        return matter_in3[index];
    }
    return matter_in0[index];
}

uint get_objects_matter(ivec2 pos) {
    int index = get_index(get_pos_inside_chunk(pos));
    int chunk_index = get_chunk_index(pos);
    if (chunk_index == 0) {
        return objects_matter0[index];
    } else if (chunk_index == 1) {
        return objects_matter1[index];
    } else if (chunk_index == 2) {
        return objects_matter2[index];
    } else if (chunk_index == 3) {
        return objects_matter3[index];
    }
    return objects_matter0[index];
}

uint get_objects_color(ivec2 pos) {
    int index = get_index(get_pos_inside_chunk(pos));
    int chunk_index = get_chunk_index(pos);
    if (chunk_index == 0) {
        return objects_color0[index];
    } else if (chunk_index == 1) {
        return objects_color1[index];
    } else if (chunk_index == 2) {
        return objects_color2[index];
    } else if (chunk_index == 3) {
        return objects_color3[index];
    }
    return objects_color0[index];
}

bool is_inside_sim_canvas(ivec2 pos) {
    ivec2 local_pos = get_local_pos(pos);
    return local_pos.x >= 0 && local_pos.x < sim_canvas_size &&
        local_pos.y >= 0 && local_pos.y < sim_canvas_size;
}

Matter read_matter(ivec2 pos) {
    uint obj_matter = get_objects_matter(pos);
    if (obj_matter != empty) {
        Matter matter = new_matter(obj_matter);
        matter.state = state_object;
        return matter;
    } else {
        return new_matter(get_matter_in(pos));
    }
}

void write_matter(ivec2 pos, Matter matter) {
    int index = get_index(get_pos_inside_chunk(pos));
    int chunk_index = get_chunk_index(pos);
    if (chunk_index == 0) {
        matter_out0[index] = matter.matter;
    } else if (chunk_index == 1) {
        matter_out1[index] = matter.matter;
    } else if (chunk_index == 2) {
        matter_out2[index] = matter.matter;
    } else if (chunk_index == 3) {
        matter_out3[index] = matter.matter;
    }
}

void write_matter_both(ivec2 pos, Matter matter) {
    int index = get_index(get_pos_inside_chunk(pos));
    int chunk_index = get_chunk_index(pos);
    if (chunk_index == 0) {
        matter_in0[index] = matter.matter;
        matter_out0[index] = matter.matter;
    } else if (chunk_index == 1) {
        matter_in1[index] = matter.matter;
        matter_out1[index] = matter.matter;
    } else if (chunk_index == 2) {
        matter_in2[index] = matter.matter;
        matter_out2[index] = matter.matter;
    } else if (chunk_index == 3) {
        matter_in3[index] = matter.matter;
        matter_out3[index] = matter.matter;
    }
}

void write_objects_matter(ivec2 pos, uint matter) {
    int index = get_index(get_pos_inside_chunk(pos));
    int chunk_index = get_chunk_index(pos);
    if (chunk_index == 0) {
        objects_matter0[index] = matter;
    } else if (chunk_index == 1) {
        objects_matter1[index] = matter;
    } else if (chunk_index == 2) {
        objects_matter2[index] = matter;
    } else if (chunk_index == 3) {
        objects_matter3[index] = matter;
    }
}

void write_image_color(ivec2 pos, vec4 color) {
    ivec2 img_pos = get_pos_inside_chunk(pos);
    int chunk_index = get_chunk_index(pos);
    if (chunk_index == 0) {
        imageStore(canvas_img0, img_pos, color);
    } else if (chunk_index == 1) {
        imageStore(canvas_img1, img_pos, color);
    } else if (chunk_index == 2) {
        imageStore(canvas_img2, img_pos, color);
    } else if (chunk_index == 3) {
        imageStore(canvas_img3, img_pos, color);
    }
}

ivec2 get_pos_at_dir(ivec2 pos, int dir) {
    return pos + OFFSETS[dir];
}

// | 0 1 2 |
// | 7 x 3 |
// | 6 5 4 |
Matter get_neighbor(ivec2 pos, int dir) {
    ivec2 neighbor_pos = get_pos_at_dir(pos, dir);
    if (is_inside_sim_canvas(neighbor_pos)) {
        return read_matter(neighbor_pos);
    } else {
        return new_matter(empty);
    }
}

Matter get_neighbor_ignore_objects(ivec2 pos, int dir) {
    ivec2 neighbor_pos = get_pos_at_dir(pos, dir);
    if (is_inside_sim_canvas(neighbor_pos)) {
        return new_matter(get_matter_in(neighbor_pos));
    } else {
        return new_matter(empty);
    }
}

bool is_object(Matter matter) {
    return matter.state == state_object;
}

bool is_solid(Matter matter) {
    return matter.state == state_solid || matter.state == state_solid_gravity;
}

bool is_gas(Matter matter) {
    return matter.state == state_gas;
}

bool is_empty(Matter matter) {
    return matter.matter == state_empty;
}

bool is_powder(Matter matter) {
    return matter.state == state_powder;
}

bool is_liquid(Matter matter) {
    return matter.state == state_liquid;
}

bool is_solid_gravity(Matter matter) {
    return matter.state == state_solid_gravity;
}

bool is_energy(Matter matter) {
    return matter.state == state_energy;
}

bool is_gravity(Matter matter) {
    return is_powder(matter) || is_liquid(matter) || is_solid_gravity(matter);
}

// For anything that falls (liquid or powder)
bool falls_on_empty(Matter from, Matter to) {
    return is_gravity(from) && is_empty(to);
}

bool falls_on_swap(Matter from, Matter to) {
    return is_gravity(from) && (is_liquid(to) || is_gas(to) || is_energy(to)) && to.weight < from.weight;
}

bool rises_on_empty(Matter from, Matter to) {
    return is_gas(from) && is_empty(to);
}

bool rises_on_swap(Matter from, Matter to) {
    return is_gas(from) && (is_liquid(to) || is_powder(to) || is_energy(to)) && to.weight > from.weight;
}

/*
For powders
    | |f| |
    |t|x| |
    f->t where x is space under f
*/
bool slides_on_empty(Matter from_diagonal, Matter to_diagonal, Matter from_down) {
    return is_powder(from_diagonal) && !is_empty(from_down) && !is_liquid(from_down) && is_empty(to_diagonal);
}

bool slides_on_swap(Matter from_diagonal, Matter to_diagonal, Matter from_down) {
    return is_powder(from_diagonal) && !is_empty(from_down) && !is_liquid(from_down) &&
    is_liquid(to_diagonal) && to_diagonal.weight < from_diagonal.weight;
}

/// From could move to one direction to empty only
bool moves_on_empty_certainly(Matter from, Matter to, Matter opposite, Matter down) {
    return push_constants.dispersion_step < from.dispersion &&
    ((is_liquid(from) && !is_empty(down)) || is_gas(from)) &&
    is_empty(to) && !is_empty(opposite);
}

/// From could move to one direction to liquid only
bool moves_on_swap_certainly(Matter from, Matter to, Matter opposite) {
    return push_constants.dispersion_step < from.dispersion &&
    (is_liquid(from) || is_gas(from)) && (is_liquid(to) || is_gas(to) || is_energy(to)) &&
    !(is_liquid(opposite) && opposite.weight < from.weight) &&
    to.weight < from.weight;
}

/// From could move to both direction to empty, but takes a change at one direction
bool moves_on_empty_maybe(Matter from, Matter to, Matter opposite, Matter down, float p) {
    return p < 0.5 && push_constants.dispersion_step < from.dispersion &&
    ((is_liquid(from) && !is_empty(down)) || is_gas(from)) &&
    is_empty(to) && is_empty(opposite);
}

/// From could move in both direction to liquid, but takes a chance at one direction
bool moves_on_swap_maybe(Matter from, Matter to, Matter opposite, float p) {
    return p < 0.5 && push_constants.dispersion_step < from.dispersion &&
    (is_liquid(from) || is_gas(from)) && (is_liquid(to) || is_gas(to)) &&
    (is_liquid(opposite) || is_gas(opposite)) && opposite.weight < from.weight &&
    to.weight < from.weight;
}

bool current_same_as_neighbors(ivec2 pos, Matter current) {
    Matter up = get_neighbor(pos, UP);
    Matter down = get_neighbor(pos, DOWN);
    Matter left = get_neighbor(pos, LEFT);
    Matter right = get_neighbor(pos, RIGHT);

    Matter up_left = get_neighbor(pos, UP_LEFT);
    Matter up_right = get_neighbor(pos, UP_RIGHT);
    Matter down_right = get_neighbor(pos, DOWN_RIGHT);
    Matter down_left = get_neighbor(pos, DOWN_LEFT);

    if (current.matter == up.matter && current.matter == down.matter &&
    current.matter == left.matter && current.matter == right.matter &&
    current.matter == up_left.matter && current.matter == up_right.matter &&
    current.matter == down_right.matter && current.matter == down_left.matter) {
        return true;
    }
    return false;
}

bool current_same_as_neighbors_ignore_objects(ivec2 pos, Matter current) {
    Matter up = get_neighbor_ignore_objects(pos, UP);
    Matter down = get_neighbor_ignore_objects(pos, DOWN);
    Matter left = get_neighbor_ignore_objects(pos, LEFT);
    Matter right = get_neighbor_ignore_objects(pos, RIGHT);

    Matter up_left = get_neighbor_ignore_objects(pos, UP_LEFT);
    Matter up_right = get_neighbor_ignore_objects(pos, UP_RIGHT);
    Matter down_right = get_neighbor_ignore_objects(pos, DOWN_RIGHT);
    Matter down_left = get_neighbor_ignore_objects(pos, DOWN_LEFT);

    if (current.matter == up.matter && current.matter == down.matter &&
    current.matter == left.matter && current.matter == right.matter &&
    current.matter == up_left.matter && current.matter == up_right.matter &&
    current.matter == down_right.matter && current.matter == down_left.matter) {
        return true;
    }
    return false;
}

vec4 color_i32_to_vec4(int c) {
    vec4 color;
    color.r = float(c & 255) / 255.0;
    color.g = float((c >> 8) & 255) / 255.0;
    color.b = float((c >> 16) & 255) / 255.0;
    color.a = float((c >> 24) & 255) / 255.0;
    return color;
}

vec4 vary_color_rgb(vec4 color, ivec2 seed_pos) {
    float seed = 0.1;
    float p = rand(seed_pos, seed);
    float variation = -0.1 + 0.2 * p;
    color.rgb += vec3(variation);
    return color;
}