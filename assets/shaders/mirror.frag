#version 450
#extension GL_ARB_separate_shader_objects : enable

layout(location = 0) in vec3 fragPos;
layout(location = 1) in vec3 fragNorm;

layout(set = 0, binding = 1) uniform UniformBufferObject {
    vec4 light_pos;
    vec4 options;
    float time;
} ubo;

layout(input_attachment_index = 0, set = 0, binding = 3) uniform subpassInput mirror_color;
layout(input_attachment_index = 0, set = 0, binding = 4) uniform subpassInput mirror_depth;

layout(location = 0) out vec4 outColor;

bool invert = bool(ubo.options[0]);
bool depth = bool(ubo.options[1]);

void main() {
    vec3 color;
    if (depth) {
        float depth = subpassLoad(mirror_depth).r;
        color = vec3(depth);
    } else {
        color = subpassLoad(mirror_color).rgb;
    }
    if (invert) {
        color = 1.0 - color;
    }
    outColor = vec4(color, 1.0);
}
