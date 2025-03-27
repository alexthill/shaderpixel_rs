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
const vec3 COLORS[] = {
    vec3(0.2),
    vec3(0.3, 0.1, 0.4),
    vec3(0.4, 0.2, 0.0)
};

float time = mod(ubo.time, 100.0);
bool invert = bool(ubo.options[2]);
bool inside = bool(ubo.options[3]);

mat2 rot2D(float th) {
    // float c = cos(th);
    // float s = sin(th);
    // return mat2(c, s, -s, c);
    vec2 a = sin(vec2(1.5707963, 0) + th);
    return mat2(a, -a.y, a.x);
}

#include "truchet.frag"

vec2 op_union(vec2 a, vec2 b) {
    return a.x < b.x ? a : b;
}

vec2 op_substraction(vec2 a, vec2 b) {
    return -a.x > b.x ? vec2(-a.x, a.y) : b;
}

float op_substraction(float a, float b) {
    return -a > b ? -a : b;
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
    p.zy *= rot2D(radians(90.));

    objId[0] = maxDist;
    objId[1] = maxDist;

    float portal = sdRotatingTorus(p, 0.05);
    return portal;
}

vec2 sdf_portal(vec3 p) {
    vec3 pos = p;
    pos.x *= 1.25;

    vec3 p_rot = vec3(pos.x, pos.yz * rot2D(radians(90)));
    float d_portal = sdf_cylinder(p_rot - vec3(0.0, 0.0, 0.25), 0.01, 0.999);
    vec2 portal = vec2(d_portal, 0.0);

    return portal;
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

vec2 raymarch_portal(vec3 pos, vec3 dir, float depth, float max_depth) {
    vec2 scene;
    for (int i = 0; i < NUM_STEPS; i++) {
        scene = sdf_portal(pos + depth * dir);
        float dist = scene.x * 0.8;
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

void main() {
    vec3 pos = inside ? cameraPos : fragPos;
    vec3 dir = normalize(fragPos - cameraPos);
    float max_depth = 100.0;
    float dim_scale = 5.0; // dimension_scale

    // portal effect variable
    railColor = vec3(1);
    ballnb = 100.0;
    railRotationSpeed = 3.0;
    railRotNb = 3.0;

    float portal_dist = raymarch_portal_effect(cameraPos, dir, 0.0, max_depth);
    vec3 portal_color = sdfColor(cameraPos + dir * portal_dist);
    if (invert) {
        portal_color = 1.0 - portal_color;
    }

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
    ballnb = clamp(ubo.options[0], 1.0, 30.0); // default is 5
    railRotationSpeed = 1.0;
    railRotNb = ubo.options[1]; // default is 3

    float depth;
    vec3 color = truchetRaymarching(pos / dim_scale, dir, depth);
    vec2 scene = raymarch_portal(pos, dir, 0.0, max_depth);

    if(portal_dist < depth * dim_scale){
        outColor = vec4(portal_color, 1.0);
        return;
    }
    if(scene.x < depth * dim_scale) {
        discard;
    }

    if (invert) {
        color = 1.0 - color;
    }
    outColor = vec4(color, 1.0); // Adding my shader here like a caveman
}
