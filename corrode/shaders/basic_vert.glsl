#version 450
layout(location=0) in vec2 position;
layout(location=1) in vec2 normal;
layout(location=2) in vec2 tex_coords;
layout(location=3) in vec4 color;

layout(push_constant) uniform PushConstants {
    mat4 world_to_screen;
    vec2 world_pos;
    mat2 rotation;
    vec4 forced_color;
    int force_color;
} push_constants;

layout(location = 0) out vec4 v_color;
layout(location = 1) out vec2 v_tex_coords;

void main() {
    gl_Position =  push_constants.world_to_screen *
        vec4(push_constants.rotation * position.xy + push_constants.world_pos, 0.0, 1.0);
    if (push_constants.force_color == 1) {
        v_color = push_constants.forced_color;
    } else {
        v_color = color;
    }
    v_tex_coords = tex_coords;
}