#version 450
#extension GL_ARB_separate_shader_objects : enable
#include "includes/lightning.glsl"

// SDF Cat by ejacquem <https://www.shadertoy.com/view/wcX3WN>

#define PI 3.1415926535

layout(location = 0) in vec3 fragPos;
layout(location = 1) in vec3 fragNorm;

layout(location = 0) out vec4 outColor;

float time = ubo.time * ubo.options[0][3];
vec3 shapeColor = ubo.options[0].xyz;
const vec3 backgroundColor = vec3(0.12);

float sdfCircle(vec2 center, float r, vec2 pos) {
    return distance(center, pos) - r;
}

float sdfBox(vec2 center, vec2 size, vec2 pos) {
        vec2 d = abs(pos - center) - size;
        return length(max(d, 0.0)) + min(max(d.x, d.y), 0.0);
}

float softMax(float a, float b, float k) {
    return log(exp(k * a) + exp(k * b)) / k;
}

float softMin(float a, float b, float k) {
    return -softMax(-a, -b, k);
}

float sdfXOR(float a, float b) {
    return min(max(a, -b), max(-a, b));
}

// intersection of two circle
float sdfPetal(vec2 pos1, vec2 pos2, float r, vec2 uv) {
    float a = sdfCircle(pos1, r, uv);
    float b = sdfCircle(pos2, r, uv);
    return max(a, b);
}

float sdfLine( vec2 p, vec2 a, vec2 b ) {
    vec2 ba = b-a;
    vec2 pa = p-a;
    float h =clamp(dot(pa,ba)/dot(ba,ba), 0.0, 1.0 );
    return length(pa-h*ba);
}

float sdfEar(vec2 center, float r, float angle, vec2 pos) {
    vec2 earPos = pos;
    mat2 rot = mat2(vec2(cos(angle), sin(angle)), vec2(-sin(angle), cos(angle)));
    earPos *= rot;

    float c1, c2;
    c1 = sdfCircle(center + vec2(30.0, 0.0), r, earPos);
    c2 = sdfCircle(center + vec2(-30.0, 0.0), r, earPos);
    float outer = max(c1, c2);

    c1 = sdfCircle(center + vec2(30.0, 0.0), r - 5.0, earPos);
    c2 = sdfCircle(center + vec2(-30.0, 0.0), r - 5.0, earPos);
    float inner = max(c1, c2);

    return softMax(outer, -inner, 0.28);
}

float sdfEars(vec2 uv) {
    float a = 0.628;
    a += sin(time + PI) * 0.1;
    float earL = sdfEar(vec2(0.0, 60.0), 60.0, a, uv);
    float earR = sdfEar(vec2(0.0, 60.0), 60.0, -a, uv);
    return min(earL, earR);
}

float sdfWhisker(vec2 uv, float a, float startX) {
    float size = 70.0;
    mat2 rotation = mat2(vec2(cos(a), sin(a)), vec2(-sin(a), cos(a)));
    vec2 A = vec2(startX + size * (startX > 0.0 ? 1.0 : -1.0), 0.0);
    vec2 B = vec2(startX, 0.0);
    vec2 offset = vec2(0.0, 10.0);
    A = A * rotation - offset;
    B = B * rotation - offset;
    return sdfLine(uv, A, B) - 1.0;
}

float sdfWhiskers(vec2 uv) {
    float a = 0.314;
    float b = 0.628;
    a += sin(time + PI) * 0.1;
    b += sin(time + PI) * 0.1;
    float startX = 20.0;
    float w1 = sdfWhisker(uv, a, startX);
    float w2 = sdfWhisker(uv, b, startX);
    float w3 = sdfWhisker(uv, PI - a, startX);
    float w4 = sdfWhisker(uv, PI - b, startX);
    return min(min(min(w1, w2), w3), w4);
}

float sdfNose(vec2 uv) {
    uv += vec2(0, sin(time + PI) + 1.0);
    float c1 = sdfCircle(vec2(-10, -14), 11.0, uv);
    float c2 = sdfCircle(vec2(10, -14), 11.0, uv);
    float c3 = sdfCircle(vec2(0, -7), 10.0, uv);
    return max(-min(c1, c2), c3);
}

float sdfCheeks(vec2 uv) {
    uv += vec2(0, sin(time + PI) + 1.0);
    vec2 offset1 = vec2(2, 2);
    vec2 offset2 = vec2(-2, 2);
    float c1, c2;
    c1 = sdfCircle(vec2(-12, -20) + offset1, 15.0, uv);
    c2 = sdfCircle(vec2(-12, -20), 15.0, uv);
    float cheekL = max(-c1, c2);
    c1 = sdfCircle(vec2(12, -20) + offset2, 15.0, uv);
    c2 = sdfCircle(vec2(12, -20), 15.0, uv);
    float cheekR = max(-c1, c2);
    return min(cheekL, cheekR);
}

float sdfEyes(vec2 uv) {
    vec2 offset1 = vec2(0.0, sin(time + PI * 2.0) + 1.0) * 2.0;
    vec2 offset2 = vec2(sin(time + PI), 0.0) * 2.0;
    float pupil, eye;
    pupil = -sdfPetal(vec2(10, 22) + offset2, vec2(32, 22) + offset2, 15.0, uv);
    eye = sdfPetal(vec2(20, 20) + offset1, vec2(25, 25), 15.0, uv);
    float eyeR = max(pupil, eye);
    pupil = -sdfPetal(vec2(-10, 22) + offset2, vec2(-32, 22) + offset2, 15.0, uv);
    eye = sdfPetal(vec2(-20, 20) + offset1, vec2(-25, 25), 15.0, uv);
    float eyeL = max(pupil, eye);
    return -min(eyeL, eyeR);
}

void main() {
    vec2 uv = fragPos.xy * 120.0;
    float body = sdfCircle(vec2(0.0), 60.0, uv);
    float ears = sdfEars(uv);
    float whiskers = sdfWhiskers(uv);
    float nose = sdfNose(uv);
    float cheeks = sdfCheeks(uv);
    float eyes = sdfEyes(uv);
    float cat;
    cat = softMin(body, ears, 0.3);
    cat = sdfXOR(cat, whiskers);
    cat = sdfXOR(cat, nose);
    cat = softMax(cat, -cheeks, 0.9);
    cat = max(cat, eyes);
    float t = clamp(cat, 0.0, 1.0);

    vec3 color  = mix(shapeColor, backgroundColor, t);
    outColor = vec4(calc_lightning(color, fragPos, normalize(fragNorm)), 1.0);
}
