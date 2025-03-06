#version 450
#extension GL_ARB_separate_shader_objects : enable

#define LN2 0.6931471805599453

layout(location = 0) in vec3 fragPos;

layout(location = 0) out vec4 outColor;

const float BAILOUT = 256.0;
const uint MAX_ITER = 100;

void main() {
    vec2 pos = fragPos.xy * 1.67;
    pos.x = -pos.x - 0.67;
    vec2 z = pos;
    vec2 zz = z*z;
    float normSqr = zz.x + zz.y;

    uint it = 0;
    while (normSqr < BAILOUT && it < MAX_ITER) {
        z = vec2(zz.x - zz.y, 2.0*z.x*z.y) + pos;
        zz = z*z;
        normSqr = zz.x + zz.y;
        it += 1;
    }

    vec3 color = vec3(-1.0);
    if (normSqr >= BAILOUT) {
        float norm = sqrt(max(0.0, (float(it) - log(log(normSqr) * 0.5)/LN2)*0.261027));
        vec3 phase = vec3(8.5, 7.9, 7.2);
        vec3 normVec = vec3(norm);
        color = sin(normVec - phase);
    }

    outColor = vec4(color / 2.0 + 0.5, 1.0);
}
