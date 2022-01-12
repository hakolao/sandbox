#version 450

layout(location=0) in vec2 v_tex_coords;
layout(location=1) in vec4 v_color;

layout(location=0) out vec4 f_color;

void main() {
    // Transform 0.0 - 1.0 to -1.0 - 1.0, center at 0.0, 0.0
    vec2 coords = 2.0 * v_tex_coords - 1.0;
    float radius_squared = 1.0;
    float radius = 1.0;
    if (dot(coords, coords) > radius_squared)
    {
        discard;
    }
    float dist = length(coords);
    float border_thickness = fwidth(dist) * 0.1;
    float alpha = smoothstep(0.0, border_thickness, abs(radius - dist));
    f_color = vec4(v_color.rgb, alpha * v_color.a);
}