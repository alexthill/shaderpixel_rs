#version 450
#extension GL_ARB_separate_shader_objects : enable

#define PI 3.1415926535

layout(location = 0) in vec3 fragPos;
layout(location = 1) in vec3 cameraPos;
layout(location = 2) in float cameraDistToContainer;

layout(set = 0, binding = 1) uniform UniformBufferObject {
    vec4 options;
    float time;
} ubo;
layout(set = 0, binding = 2) uniform sampler2D texSampler;

layout(location = 0) out vec4 outColor;

const vec4 CONTAINER_COLOR = vec4(0.0, 0.0, 0.2, 0.5);
const float SUN_RADIUS = 0.2;
const float EARTH_RADIUS = 0.1;
const float MOON_RADIUS = 0.04;

float time = ubo.time * ubo.options[0];

vec3 get_earth_pos() {
    return vec3(cos(time * 0.1), 0.0, sin(time * 0.1)) * 0.7;
}

vec3 get_moon_pos(vec3 earth_pos) {
    return earth_pos + vec3(cos(time * 0.5), 0.0, sin(time * 0.5)) * 0.2;
}

mat2 rot_mat(float angle) {
    return mat2(cos(angle), -sin(angle), sin(angle), cos(angle));
}

// blends color ca over cb if da < db, assumes pre multiplied alpha
void blend_colors(float da, vec4 ca, inout float db, inout vec4 cb) {
    if (ca.a <= 0.0) {
        return;
    }
    if (da < db) {
        db = da;
        cb = ca + cb * (1 - ca.a);
    } else {
        cb = cb + ca * (1 - cb.a);
    }
}

// calculates the distance from a point o in dir u to a sphere of center c and radius r
// returns vec3(d2 - d1, d1, d2) with d2 >= d1 or vec3(0.0) if no intersection
vec3 sphere(vec3 c, float r, vec3 o, vec3 u) {
    vec3 co = o - c;
    float b = 2.0 * dot(u, co);
    float delta = b * b - 4.0 * (dot(co, co) - r * r);
    if (delta < 0.0) {
        return vec3(0.0);
    }
    vec2 ds = -b * 0.5 + vec2(-0.5, 0.5) * sqrt(delta);
    return vec3(ds.y - ds.x, ds);
}

// same as sphere() but also computes a smooth alpha if intersecting segment is short
vec4 smooth_sphere(vec3 c, float r, vec3 o, vec3 u) {
    vec3 dists = sphere(c, r, o, u);
    vec3 mid = o + (dists.x * 0.5 + dists.y) * u;
    float d_in = r - distance(mid, c);
    return vec4(dists, smoothstep(0.0, 4.0 * dists.y / 1000.0, d_in));
}

// check if p is in the cone shadow cast by object o and light l
// assuming l and o are spheres centered at lp and op with radii lr > or
float in_shadow(vec3 lp, float lr, vec3 op, float or, vec3 p) {
    float dist_l_o = distance(lp, op);
    float dist_o_c = dist_l_o / (lr / or - 1.0);
    vec3 cone_tip = op + dist_o_c * normalize(op - lp);
    float dist_p_c = distance(p, cone_tip);

    if (dist_p_c > dist_o_c) {
        return 1.0;
    }

    vec3 n = normalize(lp - op);
    vec3 pp = (lp - p) - dot(lp - p, n) * n;
    float dist_lp_pp = length(pp);
    float theta = asin(or / dist_l_o);
    float alpha = asin(dist_lp_pp / dist_p_c);
    return smoothstep(-0.5, 0.1, (alpha - theta) / theta);
}

void sun(vec3 camera, vec3 dir, inout float d, inout vec4 color) {
    vec3 dists2 = sphere(vec3(0.0), SUN_RADIUS * 1.2, camera, dir);
    float alpha = clamp(0.0, 1.0, dists2.x * 3.0 * float(dists2.y > 0.0));
    blend_colors(dists2.z, vec4(1.0, 0.5, 0.0, 1.0) * alpha, d, color);

    vec4 dists = smooth_sphere(vec3(0.0), SUN_RADIUS, camera, dir);
    vec3 p = camera + dists.y * dir;
    float angle = dot(dir, normalize(vec3(0.0) - p));
    blend_colors(dists.y, vec4(1.0, 1.0, angle * 0.75, 1.0) * dists.w, d, color);
}

void earth(vec3 camera, vec3 dir, inout float d, inout vec4 color) {
    vec3 pos = get_earth_pos();
    vec4 dists = smooth_sphere(pos, EARTH_RADIUS, camera, dir);
    vec3 p = camera + dists.y * dir;
    float angle = dot(dir, normalize(pos - p));
    float brightness = smoothstep(-0.20, 0.2, dot(normalize(p - pos), normalize(-pos)));

    // moon shadow is a bit small, use bigger radius to make it look bigger
    brightness *= in_shadow(vec3(0.0), SUN_RADIUS, get_moon_pos(pos), MOON_RADIUS * 1.5, p);

    vec3 sp = p - pos;
    sp.xy = rot_mat(23.43602 / 180.0 * PI) * sp.xy;
    sp.xz = rot_mat(time) * sp.xz;
    float longitude = atan(sp.z, sp.x) / PI;
    float latitude = sp.y / EARTH_RADIUS;
    vec2 uv = vec2(-longitude * 0.5 + 0.5, latitude * 0.25 + 0.75);
    // fix mip mapping seam artefact at 180° by manually setting gradients
    vec2 dUVdx = dFdx(uv);
    vec2 dUVdy = dFdy(uv);
    if (abs(dUVdx.x) > 0.5) dUVdx.x = 0.0;
    if (abs(dUVdy.x) > 0.5) dUVdy.x = 0.0;
    vec4 tex_day = textureGrad(texSampler, uv, dUVdx, dUVdy);
    uv.y -= 0.5;
    vec4 tex_night = textureGrad(texSampler, uv, dUVdx, dUVdy);

    vec3 col = brightness * tex_day.xyz + (1.0 - brightness) * tex_night.xyz;
    blend_colors(dists.y, vec4(col, 1.0) * dists.w, d, color);
}

void moon(vec3 camera, vec3 dir, inout float d, inout vec4 color) {
    vec3 earth_pos = get_earth_pos();
    vec3 pos = get_moon_pos(earth_pos);
    vec4 dists = smooth_sphere(pos, MOON_RADIUS, camera, dir);
    vec3 p = camera + dists.y * dir;
    float angle = dot(dir, normalize(pos - p));
    float brightness = smoothstep(-0.20, 0.2, dot(normalize(p - pos), normalize(-pos)));

    vec3 earth_dists = sphere(earth_pos, EARTH_RADIUS, p, -normalize(p));
    if (earth_dists.x > 0.0 && earth_dists.y > 0.0) {
        brightness *= 1.0 - smoothstep(0.0, EARTH_RADIUS * 2.0, earth_dists.x) * 0.5;
    }
    brightness *= in_shadow(vec3(0.0), SUN_RADIUS, earth_pos, EARTH_RADIUS, p);

    vec3 col = brightness * vec3(0.4 + angle * 0.6);
    blend_colors(dists.y, vec4(col, 1.0) * dists.w, d, color);
}

float hash(vec3 p) {
    p = fract(p * 0.3183099 + 0.1);
    p *= 17.0;
    return fract(p.x*p.y*p.z * (p.x+p.y+p.z));
}

void stars(float dist, vec3 camera, vec3 dir, inout float d, inout vec4 color) {
    vec3 p = (camera + dist * dir) * 50;
    vec3 p_int = round(p);
    float rand = hash(p_int);
    float is_star = step(0.9, rand) * (1.0 - smoothstep(0.05, 0.25, distance(p, p_int)));
    vec4 star_color = vec4(is_star);
    blend_colors(dist, star_color, d, color);
}

void main() {
    vec3 dir = normalize(fragPos - cameraPos);
    vec4 dists = smooth_sphere(vec3(0.0), 1.0, cameraPos, dir);

    if (dists.x <= 0.0) {
        outColor = vec4(0.0);
        return;
    }

    float d = 10000.0;
    vec4 color = vec4(0.0);
    if (distance(cameraPos, vec3(0.0)) < 1.0) {
        color = CONTAINER_COLOR;
    } else {
        blend_colors(dists.z + 0.001, CONTAINER_COLOR * dists.w, d, color);
    }
    stars(dists.z, cameraPos, dir, d, color);

    moon(cameraPos, dir, d, color);
    earth(cameraPos, dir, d, color);
    sun(cameraPos, dir, d, color);

    if (distance(cameraPos, vec3(0.0)) > 1.0) {
        stars(dists.y, cameraPos, dir, d, color);
    }

    outColor = vec4(color.rgb / color.a, color.a);
}
