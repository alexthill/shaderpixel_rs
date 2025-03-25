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

const float PI2 = 6.283;
const int NUM_STEPS = 256;
const vec3 COLORS[] = {
    vec3(0.2),
    vec3(0.3, 0.1, 0.4),
    vec3(0.4, 0.2, 0.0)
};

bool inside = bool(ubo.options[3]);

mat2 rot2(float th) {
    vec2 a = sin(vec2(1.5707963, 0) + th);
    return mat2(a, -a.y, a.x);
}

vec2 op_union(vec2 a, vec2 b) {
    return a.x < b.x ? a : b;
}

vec2 op_subtraction(vec2 a, vec2 b) {
    return -a.x > b.x ? vec2(-a.x, a.y) : b;
}

float sdf_sphere(vec3 p, float s) {
    return length(p) - s;
}

float sdf_box(vec3 p, vec3 b) {
    vec3 q = abs(p) - b;
    return length(max(q, 0.0)) + min(max(q.x, max(q.y, q.z)), 0.0);
}

float sdf_cylinder(vec3 p, float h, float r) {
    vec2 d = abs(vec2(length(p.xz), p.y)) - vec2(r, h);
    return min(max(d.x, d.y), 0.0) + length(max(d, 0.0));
}

float sdf_portal_effect(vec3 p) {
    float n = 40;
    float a = atan(p.y, p.x);
    float ia = (floor(n * a / PI2) + 0.5) / n * PI2;

    p.xy *= rot2(ia);
    p.x -= 1.1;

    return sdf_sphere(abs(p), 0.05);
}

vec2 sdf_portal(vec3 p) {
    p.x *= 1.25;

    vec3 p_rot = vec3(p.x, p.yz * rot2(radians(90)));
    float d_portal = sdf_cylinder(p_rot - vec3(0.0, 0.0, 0.25), 0.01, 0.999);
    vec2 portal = vec2(d_portal, 0.0);

    vec3 q = p - vec3(0.0, -0.25, 0.0);
    q = vec3(q.xy * rot2(ubo.time * 0.5), q.z);
    float d_effect = sdf_portal_effect(q);
    portal = op_union(portal, vec2(d_effect, 2.0));

    float d_empty_box = sdf_box(p - vec3(0.0, -1.25, 0.0), vec3(1.0, 0.25, 0.2));
    portal = op_subtraction(vec2(d_empty_box, 0.0), portal);

    return portal;
}

vec2 sdf_scene(vec3 p) {
    vec3 s = vec3(2.0);
    vec3 q = p - s * clamp(round(p / s), vec3(-10.0), vec3(10.0));
    q.y += sin(round(p.z / s.z) + ubo.time) * 0.2;

    float d_sphere = sdf_sphere(q, 0.5);
    vec2 scene = vec2(d_sphere, 1.0);

    float d_empty_box = sdf_box(p, vec3(3.0, 3.0, 3.0));
    scene = op_subtraction(vec2(d_empty_box, 1.0), scene);

    if (inside) {
        scene = op_union(scene, sdf_portal(p));
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

vec2 raymarch_scene(vec3 pos, vec3 dir, float depth, float max_depth) {
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

vec2 raymarch_portal(vec3 pos, vec3 dir, float depth, float max_depth) {
    vec2 scene;
    for (int i = 0; i < NUM_STEPS; i++) {
        scene = sdf_portal(pos + depth * dir);
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

vec3 calc_light(vec3 pos, int color_id) {
    vec3 ambient_color = COLORS[color_id];
    vec3 diffuse_color = COLORS[color_id];

    inside = true; // this is needed to get normal for portal effect
    vec3 normal = estimate_normal(pos);
    vec3 light_dir = normalize(ubo.light_pos.xyz);
    float lambertian = max(dot(light_dir, normal), 0.0);

    return ambient_color + lambertian * diffuse_color;
}

void main() {
    vec3 pos = inside ? cameraPos : fragPos;
    vec3 dir = normalize(fragPos - cameraPos);
    float max_depth = 100.0;

    if (!inside) {
        vec2 portal = raymarch_portal(cameraPos, dir, 0.0, max_depth);
        if (portal.x <= 0.0) {
            outColor = vec4(COLORS[0], 0.7);
            return;
        }
        if (portal.x >= max_depth) {
            discard;
            return;
        }
        if (portal.y == 2.0) {
            outColor = vec4(calc_light(cameraPos + portal.x * dir, int(portal.y)), 1.0);
            return;
        }
    }

    vec2 scene = raymarch_scene(pos, dir, 0.0, max_depth);
    if (scene.x < max_depth) {
        if (scene.y == 0.0) {
            if (scene.x <= 0.0) {
                outColor = vec4(COLORS[0], 0.7);
            } else {
                discard;
            }
        } else {
            outColor = vec4(calc_light(pos + scene.x * dir, int(scene.y)), 1.0);
        }
    } else {
        outColor = vec4(COLORS[0], 1.0);
    }
}
