#version 450
#extension GL_ARB_separate_shader_objects : enable

layout(location = 0) in vec3 fragPos;
layout(location = 1) in vec3 cameraPos;
layout(location = 2) in float cameraDistToContainer;

layout(set = 0, binding = 1) uniform UniformBufferObject {
    vec4 light_pos;
    vec4 options;
    float time;
} ubo;

layout(location = 0) out vec4 outColor;

const int MAX_ITERS = 30;
const int MAX_STEPS = 128;
const float INSIDE_SCALE = 4.5;
const float MAX_DIST = INSIDE_SCALE * 2.0;

float scaleFactor = ubo.options[0];
int maxIterations = int(ubo.options[1]);
float epsilon = ubo.options[2];

float dist_estimate(vec3 ray_pos, float constant1, float constant2) {
    vec3 c = ray_pos;
    vec3 v = ray_pos;
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

int ray_march(vec3 pos, vec3 ray_dir, inout float dist) {
    float constant1 = abs(scaleFactor - 1.0);
    float constant2 = pow(float(abs(scaleFactor)), float(1 - maxIterations));

    for (int i = 0; i < MAX_STEPS; i++) {
        vec3 ray_pos = pos + ray_dir * dist;
        float de = dist_estimate(ray_pos, constant1, constant2);

        dist += de * 0.95;

        if (de < epsilon || dist > MAX_DIST) {
            return i + 1;
        }
    }

    return MAX_STEPS;
}

void main() {
    vec3 ray_dir = normalize(fragPos - cameraPos);
    vec3 ray_pos = (cameraPos + ray_dir * cameraDistToContainer) * INSIDE_SCALE;

    float dist = 0;
    int steps = ray_march(ray_pos, ray_dir, dist);

    if (dist >= MAX_DIST || steps == MAX_STEPS) {
        outColor = vec4(0.0, 0.0, 0.0, 0.4);
    } else {
        // The (log(epsilon) * 2.0) offset is to compensate for the fact
        // that more steps are taken when epsilon is small.
        float adjusted = max(0.0, float(steps) + log(epsilon) * 2.0);
        float adjustedMax = float(MAX_STEPS) + log(epsilon) * 2.0;

        // Sqrt increases contrast.
        float distRatio = sqrt(adjusted / adjustedMax) * 0.8;

        outColor = vec4(vec3(1.0 - distRatio), 1.0);
    }
}
