#version 450

layout(location = 0) in vec2 pos;
layout(location = 1) in vec4 col;
layout(location = 2) in vec2 tex_coords;

layout(location = 0) out vec4 colour;
layout(location = 1) out vec2 uv;


layout(set = 0, binding = 0) uniform CameraBuffer {
    mat4 view_proj;
} camera;

void main() {
    gl_Position = camera.view_proj * vec4(pos, 0.0, 1.0);
    colour = col;
    uv = tex_coords;
}
