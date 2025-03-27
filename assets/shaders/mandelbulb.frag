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

const int MAX_STEPS = 256;
const float INSIDE_SCALE = 1.2;
const float MAX_DIST = INSIDE_SCALE * 2.0;
const float BAILOUT = 4.0;

float power = ubo.options[0][0];
int maxIterations = int(ubo.options[0][1]);
float epsilon = ubo.options[0][2];
int color_index = int(ubo.options[0][3]);
bool enable_shadows = bool(ubo.options[1][0]);
bool enable_animation = bool(ubo.options[1][1]);

float sdf_scene(vec3 pos) {
    vec3 z = pos;
    float dr = 1.0;
    float r = 0.0;
    for (int i = 0; i < maxIterations; ++i) {
        r = length(z);
        if (r > BAILOUT) {
            break;
        }

        // convert to polar coordinates
        float theta = acos(z.z / r);
        float phi = atan(z.y, z.x);
        dr = pow(r, power - 1.0) * power * dr + 1.0;

        // scale and rotate the point
        float zr = pow(r, power);
        theta = theta * power;
        phi = phi * power;

        // convert back to cartesian coordinates
        z = zr * vec3(sin(theta) * cos(phi), sin(phi) * sin(theta), cos(theta));
        z += pos;
    }
    return 0.5 * log(r) * r / dr;
}

#include "includes/fractal.glsl"
#include "includes/palette.glsl"

void main() {
    vec3 dir = normalize(fragPos - cameraPos);
    vec3 pos = (cameraPos + dir * cameraDistToContainer) * INSIDE_SCALE;

    if (enable_animation)
        power = (sin(ubo.time * 0.5) * .5 + .5) * 18. + 2.0;

    float dist = 0.0;
    int steps = ray_march(pos, dir, dist);

    if (dist >= MAX_DIST) {
        outColor = vec4(0.0, 0.0, 0.0, 0.4);
    } else {
        // const vec3 ambient_color = vec3(float(steps / MAX_STEPS), 0.2, 0.4);
        const vec3 ambient_color = getPalette(length(pos + dir * dist) * 1.0, color_index);
        const vec3 diffuse_color = vec3(0.5, 0.5, 0.5);
        vec3 color = calc_lightning(pos, dir, dist, steps, ambient_color, diffuse_color);
        outColor = vec4(color, 1.0);
    }
}
