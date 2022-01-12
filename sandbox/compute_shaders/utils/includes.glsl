// Mostly same as ../simulation/includes.glsl, but with different buffer inputs
// This was separated due to macos molten vk api limiting buffers to 30

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
layout(set = 0, binding = 2) buffer BitmapBuffer { uint bitmap[]; };

/*
Matter data chunks
*/
layout(set = 0, binding = 3) buffer MatterInBuffer0 { uint matter_in0[]; };
layout(set = 0, binding = 4) buffer MatterOutBuffer0 { uint matter_out0[]; };
layout(set = 0, binding = 5) buffer ObjectsMatter0 { uint objects_matter0[]; };

layout(set = 0, binding = 6) buffer MatterInBuffer1 { uint matter_in1[]; };
layout(set = 0, binding = 7) buffer MatterOutBuffer1 { uint matter_out1[]; };
layout(set = 0, binding = 8) buffer ObjectsMatter1 { uint objects_matter1[]; };

layout(set = 0, binding = 9) buffer MatterInBuffer2 { uint matter_in2[]; };
layout(set = 0, binding = 10) buffer MatterOutBuffer2 { uint matter_out2[]; };
layout(set = 0, binding = 11) buffer ObjectsMatter2 { uint objects_matter2[]; };

layout(set = 0, binding = 12) buffer MatterInBuffer3 { uint matter_in3[]; };
layout(set = 0, binding = 13) buffer MatterOutBuffer3 { uint matter_out3[]; };
layout(set = 0, binding = 14) buffer ObjectsMatter3 { uint objects_matter3[]; };

layout(set = 0, binding = 15) buffer TmpMatter { uint tmp_matter[]; };

layout(push_constant) uniform PushConstants {
    ivec2 sim_pos_offset;
    ivec2 sim_chunk_start_offset;
} push_constants;

#include "../simulation/dirs.glsl"

const ivec2 HALF_CANVAS = ivec2(sim_canvas_size / 2);

struct Matter {
    uint matter;
    uint state;
};

Matter new_matter(uint matter) {
    Matter m;
    m.matter = matter;
    m.state = matter_state[m.matter];
    return m;
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

bool is_inside_sim_canvas(ivec2 pos) {
    ivec2 local_pos = get_local_pos(pos);
    return local_pos.x >= 0 && local_pos.x < sim_canvas_size &&
    local_pos.y >= 0 && local_pos.y < sim_canvas_size;
}

ivec2 get_pos_at_dir(ivec2 pos, int dir) {
    return pos + OFFSETS[dir];
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
