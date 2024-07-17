#version 450

layout(location = 0) in vec2 v_tex_coords;

layout(location = 0) out vec4 out_color;

layout(set = 0, binding = 1) uniform sampler2D u_texture;

void main() {
    out_color = texture(u_texture, v_tex_coords);
}
