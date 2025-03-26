#version 450
#extension GL_ARB_separate_shader_objects : enable
#include "truchet.frag"

layout(location = 0) in vec3 fragPos;
layout(location = 1) in vec3 fragNorm;
layout(location = 2) in vec3 cameraPos;

layout(set = 0, binding = 1) uniform UniformBufferObject {
    vec4 light_pos;
    vec4 options;
    float time;
} ubo;

layout(location = 0) out vec4 outColor;

// const float PI2 = 6.283;
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

vec2 op_substraction(vec2 a, vec2 b) {
    return -a.x > b.x ? vec2(-a.x, a.y) : b;
}

float op_substraction(float a, float b) {
    return -a > b ? -a : b;
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
    vec3 pos = p;
    p.x *= 1.25;
    p -= vec3(0.0, -0.25, 0.0);
    p *= 0.5;
    p.zy *= rot2(radians(90.));
    // p.xz *= rot2(ubo.time * 0.5);

    objId[0] = maxDist;
    objId[1] = maxDist;

    float portal = sdRotatingTorus(p, 0.05);
    // float d_empty_box = sdf_box(pos - vec3(0.0, -1.25, 0.0), vec3(1.0, 0.25, 0.4));
    // portal = op_substraction(d_empty_box, portal);
    return portal;
    // return sdRotatingTorus(p, 0.05);
}

vec2 sdf_portal(vec3 p) {
    vec3 pos = p;
    pos.x *= 1.25;

    vec3 p_rot = vec3(pos.x, pos.yz * rot2(radians(90)));
    float d_portal = sdf_cylinder(p_rot - vec3(0.0, 0.0, 0.25), 0.01, 0.999);
    vec2 portal = vec2(d_portal, 0.0);

    float d_effect = sdf_portal_effect(p);
    portal = op_union(portal, vec2(d_effect, 2.0));

    float d_empty_box = sdf_box(pos - vec3(0.0, -1.25, 0.0), vec3(1.0, 0.25, 0.4));
    // return vec2(d_empty_box, 2.0);

    portal = op_substraction(vec2(d_empty_box, 2.0), portal);

    return portal;
}

vec2 sdf_scene(vec3 p) {
    vec3 s = vec3(2.0);
    vec3 q = p - s * clamp(round(p / s), vec3(-00.0), vec3(00.0));
    q.y += sin(round(p.z / s.z) + ubo.time) * 0.2;

    float d_sphere = sdf_sphere(q, 0.5);
    vec2 scene = vec2(d_sphere, 1.0);

    float d_empty_box = sdf_box(p, vec3(3.0, 3.0, 3.0));
    scene = op_substraction(vec2(d_empty_box, 1.0), scene);

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

float raymarch_portal_effect(vec3 pos, vec3 dir, float depth, float max_depth) {
    vec2 scene;
    for (int i = 0; i < NUM_STEPS; i++) {
        float dist = sdf_portal_effect(pos + depth * dir);
        if (dist < depth * 0.001 || depth >= max_depth) {
            break;
        }
        depth += dist;
    }
    return depth;
}

vec2 raymarch_scene(vec3 pos, vec3 dir, float depth, float max_depth) {
    vec2 scene;
    for (int i = 0; i < NUM_STEPS; i++) {
        scene = sdf_scene(pos + depth * dir);
        float dist = scene.x;
        if (dist < depth * 0.001) {
            return vec2(depth, scene.y);
        }
        depth += dist;
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
        depth += dist;
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
    float dim_scale = 5.; // dimension_scale
    
    time = mod(ubo.time, 100.);

    railColor = vec3(1);
    ballnb = 100;
    railRotationSpeed = 2.; 
    railRotNb = 3;

    float portal_dist = raymarch_portal_effect(cameraPos, dir, 0.0, max_depth);
    vec3 portal_color = sdfColor(cameraPos + dir * portal_dist);

    if (!inside) {
        if(portal_dist < max_depth){
            outColor = vec4(portal_color, 1.0);
            return;
        }
        vec2 portal = raymarch_portal(cameraPos, dir, 0.0, max_depth);
        if (portal.x <= 0.0) {
            outColor = vec4(COLORS[0], 0.7);
            return;
        }
        if (portal.x >= max_depth) {
            discard;
            return;
        }
    }

    railColor = vec3(0);
    ballnb = 5;
    railRotationSpeed = 1.;

    float depth;
    vec3 color = truchetRaymarching(pos / dim_scale, dir, depth);
    vec2 scene = raymarch_scene(pos, dir, 0.0, max_depth);

    if(portal_dist < depth * dim_scale){
        outColor = vec4(portal_color, 1.0);
        return;
    }
    if(scene.x < depth * dim_scale)
        discard;
    outColor = vec4(color, 1.0); // Adding my shader here like a caveman
}
