#version 450
#extension GL_ARB_separate_shader_objects : enable
#include "includes/lightning.glsl"

layout(location = 0) in vec3 fragPos;
layout(location = 1) in vec3 fragNorm;

layout(location = 0) out vec4 outColor;

void main() {
    vec3 norm = normalize(fragNorm);
    vec3 color = norm;
    outColor = vec4(calc_lightning(color, fragPos, norm), 1.0);
}
