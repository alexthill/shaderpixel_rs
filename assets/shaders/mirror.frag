#version 450
#extension GL_ARB_separate_shader_objects : enable

layout(location = 0) in vec3 fragPos;
layout(location = 1) in vec3 fragNorm;

layout(set = 0, binding = 1) uniform UniformBufferObject {
    vec4 light_pos;
    vec4 options;
    float time;
} ubo;

layout(input_attachment_index = 0, set = 0, binding = 3) uniform subpassInput mirror;

layout(location = 0) out vec4 outColor;

bool invert = bool(ubo.options[0]);

void main() {
    vec3 color = subpassLoad(mirror).rgb;

    if (invert) {
        color = 1.0 - color;
    }
    outColor = vec4(color, 1.0);
}
