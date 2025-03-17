#version 450
#extension GL_ARB_separate_shader_objects : enable

layout(location = 0) in vec3 fragPos;
layout(location = 1) in vec3 fragNorm;
layout(location = 2) in vec3 cameraPos;

layout(set = 0, binding = 1) uniform UniformBufferObject {
    vec4 light_pos;
    vec4 options;
    float time;
} ubo;

layout(location = 0) out vec4 outColor;

const int NUM_STEPS = 256;
bool inside = bool(ubo.options[0]);

vec2 op_union(vec2 a, vec2 b) {
    return a.x < b.x ? a : b;
}

float sdf_sphere(vec3 p, float s) {
    return length(p) - s;
}

float sdf_box(vec3 p, vec3 b) {
    vec3 q = abs(p) - b;
    return length(max(q, 0.0)) + min(max(q.x, max(q.y, q.z)), 0.0);
}

vec2 sdf_scene(vec3 p) {
    vec3 s = vec3(2.0);
    vec3 q = p - s * clamp(round(p / s), vec3(vec2(-2.0), 2.0), vec3(vec2(2.0), 5.0));
    q.y += sin(round(p.z / s.z) + ubo.time) * 0.2;

    float d_sphere = sdf_sphere(q, 0.5);
    vec2 scene = vec2(d_sphere, 0.0);

    if (inside) {
        float d_portal = sdf_box(p - vec3(0.0, 0.0, 0.0), vec3(1.0, 2.0, 0.01));
        scene = op_union(scene, vec2(d_portal, 1.0));
    }

    return scene;
}

vec3 estimate_normal(vec3 p) {
    const float eps = 0.0001;
    return normalize(vec3(
        sdf_scene(vec3(p.x + eps, p.y, p.z)).x - sdf_scene(vec3(p.x - eps, p.y, p.z)).x,
        sdf_scene(vec3(p.x, p.y + eps, p.z)).x - sdf_scene(vec3(p.x, p.y - eps, p.z)).x,
        sdf_scene(vec3(p.x, p.y, p.z + eps)).x - sdf_scene(vec3(p.x, p.y, p.z - eps)).x
    ));
}

vec2 raymarch(vec3 pos, vec3 dir, float depth, float max_depth) {
    vec2 scene;
    for (int i = 0; i < NUM_STEPS; i++) {
        scene = sdf_scene(pos + depth * dir);
        float dist = scene.x;
        if (dist < depth * 0.001) {
            return vec2(depth, scene.y);
        }
        depth += dist * 0.5;
        if (depth >= max_depth) {
            return vec2(max_depth, 1.0);
        }
    }
    return vec2(depth, scene.y);
}


void main() {
    vec3 dir = normalize(fragPos - cameraPos);
    float max_depth = 100.0;
    vec2 scene = raymarch(cameraPos, dir, 0.0, max_depth);

    if (scene.x < max_depth) {
        if (scene.y == 1.0) {
            outColor = vec4(0.0);
            return;
        }
        vec3 ambient_color = vec3(0.4, 0.2, 0.1);
        vec3 diffuse_color = vec3(0.4, 0.2, 0.1);

        vec3 normal = estimate_normal(cameraPos + scene.x * dir);
        vec3 light_dir = normalize(ubo.light_pos.xyz);
        float lambertian = max(dot(light_dir, normal), 0.0);
        vec3 color = ambient_color + lambertian * diffuse_color;

        outColor = vec4(color, 1.0);
    } else {
        outColor = vec4(vec3(0.2), 1.0);
    }
}
