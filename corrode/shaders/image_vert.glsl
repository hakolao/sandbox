#version 450
layout(location=0) in vec2 position;
layout(location=1) in vec2 normal;
layout(location=2) in vec2 tex_coords;
layout(location=3) in vec4 color;

layout(push_constant) uniform PushConstants {
    mat4 world_to_screen;
    vec2 world_pos;
    mat2 rotation;
    vec2 dims;
    int invert_y;
} push_constants;

layout(location = 0) out vec2 f_tex_coords;

void main() {
    float invert_y = 1.0;
    if (push_constants.invert_y == 1) {
        invert_y *= -1.0;
    }
    gl_Position =  push_constants.world_to_screen *
    vec4(push_constants.rotation *
        vec2(position.x * push_constants.dims.x, position.y * push_constants.dims.y * invert_y) +
        push_constants.world_pos, 0.0, 1.0);
    f_tex_coords = tex_coords;
}