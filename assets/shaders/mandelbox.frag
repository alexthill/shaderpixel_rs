#version 450
#extension GL_ARB_separate_shader_objects : enable

layout(location = 0) in vec3 fragPos;
layout(location = 1) in vec3 cameraPos;
layout(location = 2) in float cameraDistToContainer;

layout(set = 0, binding = 1) uniform UniformBufferObject {
    vec4 light_pos;
    vec4 options[2];
    float time;
} ubo;

layout(location = 0) out vec4 outColor;

const int MAX_ITERS = 30;
const int MAX_STEPS = 128;
const float INSIDE_SCALE = 4.5;
const float MAX_DIST = INSIDE_SCALE * 2.0;

float scaleFactor = ubo.options[0][0];
int maxIterations = int(ubo.options[0][1]);
float epsilon = ubo.options[0][2];
bool enable_shadows = bool(ubo.options[0][3]);

float constant1 = abs(scaleFactor - 1.0);
float constant2 = pow(float(abs(scaleFactor)), float(1 - maxIterations));

float sdf_scene(vec3 pos) {
    vec3 c = pos;
    vec3 v = pos;
    float dr = 1.0;

    for (int i = 0; i < MAX_ITERS; i++) {
        if (i == maxIterations) {
            break;
        }

        // Box fold
        v = clamp(v, -1.0, 1.0) * 2.0 - v;

        // Sphere fold
        float mag = dot(v, v);
        if (mag < 0.25) {
            v = v * 4.0;
            dr = dr * 4.0;
        } else if (mag < 1.0) {
            v = v / mag;
            dr = dr / mag;
        }

        v = v * scaleFactor + c;
        dr = dr * abs(scaleFactor) + 1.0;
    }

    return (length(v) - constant1) / dr - constant2;
}

#include "includes/fractal.glsl"

void main() {
    vec3 dir = normalize(fragPos - cameraPos);
    vec3 pos = (cameraPos + dir * cameraDistToContainer) * INSIDE_SCALE;

    float dist = 0.0;
    int steps = ray_march(pos, dir, dist);

    if (dist >= MAX_DIST || steps == MAX_STEPS) {
        outColor = vec4(0.0, 0.0, 0.0, 0.4);
    } else {
        const vec3 ambient_color = vec3(0.5, 0.4, 0.4);
        const vec3 diffuse_color = vec3(0.4, 0.4, 0.5);
        vec3 color = calc_lightning(pos, dir, dist, steps, ambient_color, diffuse_color);
        outColor = vec4(color, 1.0);
    }
}
