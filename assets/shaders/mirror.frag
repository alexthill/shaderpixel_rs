#version 450
#extension GL_ARB_separate_shader_objects : enable

layout(location = 0) in vec3 fragPos;
layout(location = 1) in vec3 fragNorm;

layout(input_attachment_index = 0, set = 0, binding = 3) uniform subpassInput mirror;

layout(location = 0) out vec4 outColor;

void main() {
    outColor = vec4(subpassLoad(mirror).rgb, 1.0);
}
