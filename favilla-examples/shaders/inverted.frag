#version 450
#extension GL_ARB_separate_shader_objects : enable


layout(location = 0) in vec4 colour;
layout(location = 1) in vec2 uv;

layout(location = 0) out vec4 outColor;

layout (set = 1, binding = 0) uniform sampler2D sprite[2];

void main() {
    vec4 textureSample = texture(sprite[0], uv);
    outColor = vec4(vec3(1.0, 1.0, 1.0) - textureSample.xyz, textureSample.w);
}
