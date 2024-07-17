#version 450

layout(location = 0) in vec3 a_position;
layout(location = 1) in vec2 a_tex_coords;

layout(location = 0) out vec2 v_tex_coords;

layout(set = 0, binding = 0) uniform Transform {
    mat4 model;
    mat4 view;
    mat4 proj;
} transform;

void main() {
    v_tex_coords = a_tex_coords;
    gl_Position = transform.proj * transform.view * transform.model * vec4(a_position, 1.0);
}

