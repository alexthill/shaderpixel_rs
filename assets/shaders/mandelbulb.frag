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

const int MAX_STEPS = 256;
const float INSIDE_SCALE = 1.2;
const float MAX_DIST = INSIDE_SCALE * 2.0;
const float BAILOUT = 4.0;

float power = ubo.options[0];
int maxIterations = int(ubo.options[1]);
float epsilon = ubo.options[2];

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

vec3 estimate_normal(vec3 p) {
    const float eps = 0.0001;
    return normalize(vec3(
        sdf_scene(vec3(p.x + eps, p.y, p.z)).x - sdf_scene(vec3(p.x - eps, p.y, p.z)).x,
        sdf_scene(vec3(p.x, p.y + eps, p.z)).x - sdf_scene(vec3(p.x, p.y - eps, p.z)).x,
        sdf_scene(vec3(p.x, p.y, p.z + eps)).x - sdf_scene(vec3(p.x, p.y, p.z - eps)).x
    ));
}

int ray_march(vec3 pos, vec3 ray_dir, inout float dist) {
    for (int i = 0; i < MAX_STEPS; i++) {
        vec3 ray_pos = pos + ray_dir * dist;
        float de = sdf_scene(ray_pos);

        dist += de * 0.5;

        // increase epsilon with number of iterations to reduce aliasing
        if (de < epsilon * sqrt(float(i)) || dist > MAX_DIST) {
            return i + 1;
        }
    }

    return MAX_STEPS;
}

void main() {
    vec3 dir = normalize(fragPos - cameraPos);
    vec3 pos = (cameraPos + dir * cameraDistToContainer) * INSIDE_SCALE;

    float dist = 0.0;
    int steps = ray_march(pos, dir, dist);

    if (dist >= MAX_DIST) {
        outColor = vec4(0.0, 0.0, 0.0, 0.4);
    } else {
        const vec3 back_pos = pos + dir * dist;
        const vec3 ambient_color = vec3(float(steps / MAX_STEPS), 0.2, 0.4);
        const vec3 diffuse_color = vec3(0.4, 0.2, 0.2);

        // The (log(epsilon) * 2.0) offset is to compensate for the fact
        // that more steps are taken when epsilon is small.
        float adjusted = max(0.0, float(steps) + log(epsilon) * 2.0);
        float adjustedMax = float(MAX_STEPS) + log(epsilon) * 2.0;
        // Sqrt increases contrast.
        float distRatio = sqrt(adjusted / adjustedMax) * 0.8;

        vec3 normal = estimate_normal(back_pos);
        vec3 light_dir = normalize(ubo.light_pos.xyz);
        float lambertian = max(dot(light_dir, normal), 0.0);

        dist = 0.0;
        ray_march(back_pos, light_dir, dist);
        float shadow = dist < MAX_DIST ? 0.1 : 1.0;

        vec3 color = ambient_color * (1 - distRatio)
            + diffuse_color * lambertian * shadow;

        outColor = vec4(color, 1.0);
    }
}
