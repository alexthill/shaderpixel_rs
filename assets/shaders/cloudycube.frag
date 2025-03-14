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

const vec4 bgColor = vec4(0.15, 0.69, 0.86, 1.0);
vec3 lightDir = normalize(ubo.light_pos.xyz);
const float cloudSize = 1.0;

const vec3 RAYLEIGH = 1.0 - vec3(0.9451, 0.8314, 0.7961);

const vec3 sigmaScattering = 100.0 * RAYLEIGH;
const vec3 sigmaExtinction = 200.0 * RAYLEIGH;

mat2 rot2D(float angle)
{
    float c = cos(angle);
    float s = sin(angle);
    return mat2(c, s, -s, c);
}

// https://www.shadertoy.com/view/3s3GDn
float getGlow(float dist, float radius, float intensity){
	return max(0.0, pow(radius/max(dist, 1e-5), intensity));	
}

// https://gist.github.com/DomNomNom/46bb1ce47f68d255fd5d
// Compute the near and far intersections using the slab method.
// No intersection if tNear > tFar.
vec2 intersectAABB(vec3 rayOrigin, vec3 rayDir, vec3 boxMin, vec3 boxMax) {
    vec3 tMin = (boxMin - rayOrigin) / rayDir;
    vec3 tMax = (boxMax - rayOrigin) / rayDir;
    vec3 t1 = min(tMin, tMax);
    vec3 t2 = max(tMin, tMax);
    float tNear = max(max(t1.x, t1.y), t1.z);
    float tFar = min(min(t2.x, t2.y), t2.z);
    return vec2(tNear, tFar);
}

// return the normal of the surface the ray intersected
vec3 intersectAABBNorm(vec3 rayOrigin, vec3 rayDir, vec3 boxMin, vec3 boxMax) {
    vec2 nearFar = intersectAABB(rayOrigin, rayDir, boxMin, boxMax);
    vec3 hit = rayOrigin + rayDir * nearFar.x;
    if (abs(hit.x) >= abs(hit.y) && abs(hit.x) >= abs(hit.z))
        return vec3(sign(hit.x), 0, 0);
    if (abs(hit.y) >= abs(hit.x) && abs(hit.y) >= abs(hit.z))
        return vec3(0, sign(hit.y), 0);
    return vec3(0, 0, sign(hit.z));
}

// return the normal of the surface the ray intersected
vec3 squareNorm(vec3 pos, vec3 boxMin, vec3 boxMax) {
    if (abs(pos.x) > abs(pos.y) && abs(pos.x) > abs(pos.z))
        return vec3(sign(pos.x), 0, 0);
    if (abs(pos.y) > abs(pos.x) && abs(pos.y) > abs(pos.z))
        return vec3(0, sign(pos.y), 0);
    return vec3(0, 0, sign(pos.z));
}

const float den1 = 0.025;
const float den2 = 0.1;
const float scale = 2.0;
const float bigCircleSize = 0.4 * scale;
const float smallCircleSize = 0.3 * scale;
const float circleWidth = 0.05 * scale;

// float sampleDensity(vec3 pos){
//     // pos.xy *= rot2D(u_time);
//     // pos.zy *= rot2D(u_time);
//     if (length(pos) < smallCircleSize)
//         return den1;
//     if (length(pos) < bigCircleSize){
//         if (length(pos.xxx) < circleWidth)
//             return den2;
//         if (length(pos.yyy) < circleWidth)
//             return den2;
//         if (length(pos.zzz) < circleWidth)
//             return den2;
//     }
//     return den1;
// }
mat2 rot = rot2D(ubo.time);
float sampleDensity(vec3 pos) {
    pos.xy *= rot;
    pos.zy *= rot;
    float lenSq = dot(pos, pos);

    if (lenSq < smallCircleSize * smallCircleSize)
        return den1;

    if (lenSq < bigCircleSize * bigCircleSize) {
        float circleWidthSq = circleWidth * circleWidth;

        if (pos.x*pos.x < circleWidthSq || pos.y*pos.y < circleWidthSq || pos.z*pos.z < circleWidthSq)
            return den2;
    }
    return den1;
}

vec3 lightRay(vec3 rayOrigin, vec3 rayDir)
{
    vec2 nearFar = intersectAABB(rayOrigin, rayDir, vec3(-cloudSize), vec3(cloudSize));
    if(nearFar.x >= nearFar.y) return vec3(0.);
    vec3 pos = vec3(0);
    float stepSize = 0.05;
    float t = stepSize; // dist along the ray
    vec3 accumulatedDensity = vec3(0.0);
    vec3 sigmaStep = sigmaExtinction * stepSize;

    for (int i = 1; i <= 20; i++)
    {
        pos = rayOrigin + rayDir * t;
        accumulatedDensity += sampleDensity(pos) * sigmaStep;
        t += stepSize;
        if (t >= nearFar.y)
            break;
    }
    return exp(-accumulatedDensity) * 10.0;
}

float HenyeyGreenstein(float g, float costh){
	return (1.0 / (4.0 * 3.1415))  * ((1.0 - g * g) / pow(1.0 + g*g - 2.0*g*costh, 1.5));
}

void raymarch(vec3 rayOrigin, vec3 rayDir, inout vec3 transmittance, inout vec3 scatteredLight)
{
    vec2 nearFar = intersectAABB(rayOrigin, rayDir, vec3(-cloudSize), vec3(cloudSize));
    if(nearFar.x >= nearFar.y) return;
    vec3 pos = vec3(0);
    // vec3 prev_pos = pos;
    float density = den1; 
    float prev_density = den1;
    float stepSize = 0.015;
    float t = max(0.0, nearFar.x); // tot dist from ray origin (starts at near intersection)
    float prev_t = t;
    bool backtracking = false;

    float ambient = 0.0;
    float phase = HenyeyGreenstein(0.001, dot(rayDir, lightDir));
    phase = 0.2;

    int refractionNb = 0;
    float refractionLoss = 1.0;

    for (int i = 0; i < 300; i++)
    {
        pos = rayOrigin + rayDir * t;
        prev_density = density;
        density = sampleDensity(pos);

        vec3 SigmaS = sigmaScattering * density;
        vec3 SigmaE = sigmaExtinction * density;

        vec3 S = (lightRay(pos,lightDir) * phase + ambient) * SigmaS;
        vec3 Tr = exp(-SigmaE * stepSize);
        vec3 Sint = (S - S * Tr) / SigmaE;
        scatteredLight += transmittance * Sint * refractionLoss;
        transmittance *= Tr;

        prev_t = t;
        t += stepSize;
        if(t > nearFar.y){
            refractionNb++;
            rayOrigin = pos;
            rayDir = reflect(rayDir, squareNorm(pos, vec3(-cloudSize), vec3(cloudSize)));
            nearFar = intersectAABB(rayOrigin, rayDir, vec3(-cloudSize), vec3(cloudSize));
            refractionLoss *= 0.5;
            t = nearFar.x;
        }
        if(refractionNb > 5)
            break;
    }
    return;
}

float diffuse(vec3 normal, vec3 lightDir){
    return max(dot(normal, lightDir), 0.0);
}

float specular(vec3 rayDir, vec3 normal, vec3 lightDir){
    vec3 reflectDir = reflect(lightDir, normal);  

    float spec = pow(max(dot(rayDir, reflectDir), 0.0), 128.);
    return 3.0 * spec;
}

void main(){

    vec3 rayOrigin = cameraPos;
    vec3 rayDir = normalize(fragPos - cameraPos);

    vec2 nearFar = intersectAABB(rayOrigin, rayDir, vec3(-cloudSize), vec3(cloudSize));
    vec3 normal = intersectAABBNorm(rayOrigin, rayDir, vec3(-cloudSize), vec3(cloudSize));
    float dif = 0.0;
    float spec = 0.0;
    vec3 refractRayDir = rayDir;
    if(nearFar.x > 0.0){
        dif = diffuse(normal, lightDir);
        spec = specular(rayDir, normal, lightDir);
        refractRayDir = refract(rayDir, normal, 0.97);
    }

    vec3 scatteredLight = vec3(0.0);
    vec3 transmittance = vec3(1.0);

    if(nearFar.x < nearFar.y)
        raymarch(rayOrigin, refractRayDir, transmittance, scatteredLight);

    vec3 background = bgColor.rgb;

    float mu = dot(rayDir, lightDir);
    background += getGlow(1.0-mu, 0.00015, 1.0);

    vec4 color = vec4(vec3(background * transmittance + scatteredLight), 1.0);

    if (transmittance.x < 1.0)
        color += dif * 0.2 + spec * 0.2;

    outColor = color;
}