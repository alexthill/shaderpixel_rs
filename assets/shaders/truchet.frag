const float PI2 = 6.283185;

const float maxDist = 10.0;
const float epsilon = 0.001;
const vec4 bgColor = vec4(1.0);
const int steps = 100;
const vec3 lightDir = normalize(vec3(1.2, 1, -1.1));
const mat2 rot90 = mat2(0, 1, -1, 0);

vec3 railColor = vec3(0);
float ballnb = 5.0;
float railRotNb = 3.0;
float railRotationSpeed = 1.0;
vec2 objId;

float hash13(vec3 p3) {
    p3 = fract(p3 * 0.1031);
    p3 += dot(p3, p3.zyx + 31.32);
    return fract((p3.x + p3.y) * p3.z);
}

float sdfSphere(vec3 pos, float s) {
    return length(pos) - s;
}

float opSmoothUnion(float d1, float d2, float k) {
    float h = clamp(0.5 + 0.5 * (d2 - d1) / k, 0.0, 1.0);
    return mix(d2, d1, h) - k * h * (1.0 - h);
}

// Mobius equation from https://www.shadertoy.com/view/XldSDs
const float toroidRadius = 0.5; // The object's disc radius.
float sdfMobius(vec3 p) {
    float a = atan(p.z, p.x);
    p.xz *= rot2D(a);
    p.x -= toroidRadius;
    p.xy *= rot2D(a * railRotNb + time * railRotationSpeed);

    p = abs(abs(p) - rail_size); // 0.06
    return sdfSphere(p, rail_size + rail_width - 0.01); // 0.061
}

float sdfSphereTorus(vec3 p){
    float a = atan(p.z, p.x);
    float ball = ballnb * 4.0;
    float ia = (floor(ball * a / PI2) + 0.5) / ball * PI2;

    p.xz *= rot2D(ia);
    p.x -= toroidRadius;

    return sdfSphere(abs(p), ball_size);
}

float sdRotatingTorus(vec3 p, float k) {
    float sdfS, sdfT;

    sdfT = sdfMobius(p);
    p.xz *= rot2D(time * 0.1);
    sdfS = sdfSphereTorus(p);

    objId[0] = min(sdfS, objId[0]);
    objId[1] = min(sdfT, objId[1]);

    return opSmoothUnion(sdfS, sdfT, k);
}

float sdRotatingTorus(vec3 pos) {
    return sdRotatingTorus(pos, 0.0);
}

float sdfMap(vec3 pos) {
    // switching axis on a checkerboard pattern
    // learned from: https://www.shadertoy.com/view/MtSyRz
    {
        vec3 sn = sign(mod(floor(pos), 2.0) - 0.5);
        pos.xz *= sn.y;
        pos.xy *= sn.z;
        pos.zy *= sn.x;
    }

    vec3 fpos = fract(pos) - 0.5;
    float sdf = maxDist;
    float d = 0.5; // circle offset
    vec3 p;

    objId[0] = maxDist;
    objId[1] = maxDist;

    p = fpos + vec3(d,0,d);
    sdf = min(sdf, sdRotatingTorus(p));

    p = fpos + vec3(0,d,-d);
    p.xy *= rot90;
    sdf = min(sdf, sdRotatingTorus(p));

    p = fpos + vec3(-d,-d,0);
    p.zy *= -rot90;
    sdf = min(sdf, sdRotatingTorus(p));

    return sdf;
}

float trilinearInterpolation(vec3 p) {
    vec3 gridPos = floor(p);
    vec3 frac = fract(p);

    // sample the 8 surrounding points
    float c000 = hash13(gridPos + vec3(0,0,0));
    float c100 = hash13(gridPos + vec3(1,0,0));
    float c010 = hash13(gridPos + vec3(0,1,0));
    float c110 = hash13(gridPos + vec3(1,1,0));
    float c001 = hash13(gridPos + vec3(0,0,1));
    float c101 = hash13(gridPos + vec3(1,0,1));
    float c011 = hash13(gridPos + vec3(0,1,1));
    float c111 = hash13(gridPos + vec3(1,1,1));

    float c00 = mix(c000, c100, frac.x);
    float c01 = mix(c001, c101, frac.x);
    float c10 = mix(c010, c110, frac.x);
    float c11 = mix(c011, c111, frac.x);

    float c0 = mix(c00, c10, frac.y);
    float c1 = mix(c01, c11, frac.y);

    return mix(c0, c1, frac.z);
}

vec3 get3dColorGradient(vec3 pos){
    return getPalette(trilinearInterpolation(pos + time * 0.2) * 2.0, color_index);
    // return palette(trilinearInterpolation(pos + time * 0.2) * 2.0, PAL3);
}

vec3 raymarch(vec3 rayOrigin, vec3 rayDir){
    float m_dist = maxDist;
    float t = 0.0; // total dist
    vec3 pos = vec3(0);
    vec3 startPos = rayOrigin;

    for (int i = 0; i < steps; i++){
        pos = startPos + rayDir * t;
        m_dist = sdfMap(pos);

        if (m_dist < epsilon || t > maxDist) {
            break;
        }

        t += m_dist;
    }
    return vec3(pos);
}

vec3 sdfColor(vec3 pos){
    if (objId[0] < objId[1]) {
        return get3dColorGradient(pos);
    }
    float d = abs(objId[0] - objId[1]);
    vec3 c1 = get3dColorGradient(pos);
    vec3 c2 = railColor;
    return max(vec3(0.0), mix(c1, c2, d * 22.0));
}

vec3 truchetRaymarching(vec3 rayOrigin, vec3 rayDir, out float dist){
    vec3 pos = raymarch(rayOrigin, rayDir);
    dist = distance(pos, rayOrigin);
    float depth = 1.0 - (dist / maxDist);
    vec3 sphereColor = sdfColor(pos);
    vec3 color = sphereColor * depth;
    return color;
}
