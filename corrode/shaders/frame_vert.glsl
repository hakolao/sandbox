#version 450
layout(location=0) in vec2 position;
layout(location=1) in vec2 tex_coords;

layout(push_constant) uniform PushConstants {
    int invert_y;
} push_constants;

layout(location = 0) out vec2 f_tex_coords;

void main() {
    float invert_y = 1.0;
    if (push_constants.invert_y == 1) {
        invert_y *= -1.0;
    }
    gl_Position =  vec4(vec2(position.x, invert_y * position.y), 0.0, 1.0);
    f_tex_coords = tex_coords;
}