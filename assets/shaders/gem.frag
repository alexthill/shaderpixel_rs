#version 450
#extension GL_ARB_separate_shader_objects : enable

layout(location = 0) in vec3 fragPos;
layout(location = 1) in vec3 cameraPos;
layout(location = 2) in float cameraDistToContainer;

layout(set = 0, binding = 1) uniform UniformBufferObject {
    vec4 light_pos;
    vec4 options[2];
    float time;
} ubo;

layout(location = 0) out vec4 outColor;

const float maxDist = 1000.;
const float epsilon = 0.001;
const int steps = 500;

vec3 lightDir = normalize(ubo.light_pos.xyz);
const vec3 gemColor = vec3(0.78, 0.19, 0.19);
int gemType = int(ubo.options[0][0]);
int colorIndex = int(ubo.options[0][1]); // 0 is default unicolor
float rotationSpeed = ubo.options[0][2];
bool enable_diffuse = bool(ubo.options[0][3]);
bool enable_specular = bool(ubo.options[1][0]);

#define PAL1 vec3(0.5,0.5,0.5),vec3(0.5,0.5,0.5),vec3(1.0,1.0,1.0),vec3(0.0,0.33,0.67)
#define PAL2 vec3(0.5,0.5,0.5),vec3(0.5,0.5,0.5),vec3(1.0,1.0,1.0),vec3(0.0,0.10,0.20) 
#define PAL3 vec3(0.5,0.5,0.5),vec3(0.5,0.5,0.5),vec3(1.0,1.0,1.0),vec3(0.3,0.20,0.20)
#define PAL4 vec3(0.5,0.5,0.5),vec3(0.5,0.5,0.5),vec3(1.0,1.0,0.5),vec3(0.8,0.90,0.30)
#define PAL5 vec3(0.5,0.5,0.5),vec3(0.5,0.5,0.5),vec3(1.0,0.7,0.4),vec3(0.0,0.15,0.20)
#define PAL6 vec3(0.5,0.5,0.5),vec3(0.5,0.5,0.5),vec3(2.0,1.0,0.0),vec3(0.5,0.20,0.25)
#define PAL7 vec3(0.8,0.5,0.4),vec3(0.2,0.4,0.2),vec3(2.0,1.0,1.0),vec3(0.0,0.25,0.25)

// https://iquilezles.org/articles/palettes/
// cosine based palette, 4 vec3 params
vec3 palette(float t,vec3 a,vec3 b,vec3 c,vec3 d )
{
    return a + b*cos( 6.283185*(c*t+d) );
}

vec3 getPalette(float t){
    if (colorIndex == 0)
        return 1.0 - gemColor;
    else if (colorIndex == 1)
        return palette(t, PAL1);
    else if (colorIndex == 2)
        return palette(t, PAL2);
    else if (colorIndex == 3)
        return palette(t, PAL3);
    else if (colorIndex == 4)
        return palette(t, PAL4);
    else if (colorIndex == 5)
        return palette(t, PAL5);
    else if (colorIndex == 6)
        return palette(t, PAL6);
    else if (colorIndex == 7)
        return palette(t, PAL7);
    return 1.0 - gemColor;
}

// https://www.shadertoy.com/view/3s3GDn
float getGlow(float dist, float radius, float intensity){
	return max(0.0, pow(radius/max(dist, 1e-5), intensity));	
}

mat2 rot2D(float angle)
{
    float c = cos(angle);
    float s = sin(angle);
    return mat2(c, s, -s, c);
}

float sdfOctahedron( vec3 p, float s)
{
  p = abs(p);
  return (p.x+p.y+p.z-s)*0.57735027;
}

float sdfSphere(vec3 pos, float s)
{
  return length(pos) - s;
}

// return the given sdf cut by the plane
float sdfPlaneCut(float sdf, vec3 p, vec3 n, float h)
{
  // n must be normalized
  return max(sdf, dot(p,n) + h);
}

#define N normalize

float sdfCustomGem(vec3 p){
    p.y += 0.5;
    p = abs(p);
    p.xz = abs(p.xz);
    // d = sdfPlaneCut(d, p, vec3(-1, 0, 0), -0.0); 
    // d = sdfPlaneCut(d, p, vec3(+0, 0, -1), -0.0);
    float scale = 0.13;
    float angle = -45.;
    float d = sdfSphere(p, 6.6 * scale);

    float h = -2.0 * scale;
    d = sdfPlaneCut(d, p, vec3(0, +1, 0), h);  // top cut


    vec3 q = p;
    h = -4.73 * scale;
    d = sdfPlaneCut(d, q, N(vec3(+0, 1, 1)), h);
    q.xz *= rot2D(radians(angle));
    d = sdfPlaneCut(d, q, N(vec3(+0, 1, 1)), h);
    q.xz *= rot2D(radians(angle));
    d = sdfPlaneCut(d, q, N(vec3(+0, 1, 1)), h);

    q = p;
    h = -3.5 * scale;
    q.xz *= rot2D(radians(angle / 2.0));
    d = sdfPlaneCut(d, q, N(vec3(0, 2, 1)), h);
    q.xz *= rot2D(radians(angle));
    d = sdfPlaneCut(d, q, N(vec3(0, 2, 1)), h);

    q = p;
    h = -2.89 * scale;
    d = sdfPlaneCut(d, q, N(vec3(0, 3.5, 1)), h);
    q.xz *= rot2D(radians(angle));
    d = sdfPlaneCut(d, q, N(vec3(0, 3.5, 1)), h);
    q.xz *= rot2D(radians(angle));
    d = sdfPlaneCut(d, q, N(vec3(0, 3.5, 1)), h);

    return d;
}

float sdfMap(vec3 pos)
{
    pos.zx *= rot2D(ubo.time * rotationSpeed);
    pos.y += sin(ubo.time * rotationSpeed * 2.0) * 0.15;
    if (gemType == 0)
        return sdfCustomGem(pos);
    vec3 p = pos;
    float offset = 3.5; 
    float scale = 0.15;

    float s1 = sdfOctahedron(p, 5. * scale);

    p = pos;
    p.y += offset * scale;
    float s2 = sdfOctahedron(p, 2. * scale);
    p = pos;
    p.y -= offset * scale;
    float s3 = sdfOctahedron(p, 2. * scale);

    return min(min(s1, s2),s3);
}

vec3 calculateNormal(vec3 pos)
{
    vec2 e = vec2(1.0,-1.0)*0.5773*0.0001;
    return normalize( e.xyy*sdfMap( pos + e.xyy ) + 
					  e.yyx*sdfMap( pos + e.yyx ) + 
					  e.yxy*sdfMap( pos + e.yxy ) + 
					  e.xxx*sdfMap( pos + e.xxx ) );
}

float diffuse(vec3 normal, vec3 lightDir){
    return max(dot(normal, lightDir), 0.0);
}

float specular(vec3 rayDir, vec3 normal, vec3 lightDir, float po){
    vec3 reflectDir = reflect(lightDir, normal);  

    float spec = pow(max(dot(rayDir, reflectDir), 0.0), po);
    return spec;
}

const int maxReflection = 5;

vec3 raymarch(vec3 rayOrigin, vec3 rayDir, inout vec3 transmittance, inout vec3 scatteredLight)
{
    float m_dist = maxDist;
    float t = 0.0; // total dist
    vec3 pos = rayOrigin;
    float prev_dist;
    vec3 prev_pos;
    int reflexion = 0;
    float refractionLoss = 1.0;
    bool first = true;
    vec3 originalRayDir = rayDir;

    vec3 SigmaE = 1.0 - gemColor;

    for (int i = 0; i < steps; i++){
        prev_pos = pos;
        pos = rayOrigin + rayDir * t;
        prev_dist = m_dist;
        m_dist = sdfMap(pos);

        if (m_dist < 0.0){
            // return calculateNormal(pos);
            float density = 0.1 / refractionLoss;
            vec3 Tr = exp(-SigmaE * density * abs(m_dist));
            transmittance *= Tr;
        }

        if (sign(prev_dist) != sign(m_dist)){ // ray went through surface if sign flip
            vec3 normal = calculateNormal(pos);
            float dif = diffuse(normal, lightDir) * 0.1;
            if (prev_dist < 0.){ // if ray was inside the object, reflect
                reflexion++;
                if (reflexion > maxReflection)
                    break;
                rayDir = reflect(rayDir, normal);
                rayOrigin = prev_pos;
                t = 0.;
                SigmaE = getPalette(rayDir.x);
                refractionLoss *= .45;
                continue;
            }
            else if (first){
                first = false;
                if (rayOrigin == prev_pos) // if ray start inside the gem, don't refract
                    continue;
                rayDir = refract(rayDir, normal, 0.95);
                originalRayDir = rayDir;

                rayOrigin = prev_pos;
                t = 0.;
                // float spec = specular(rayDir, normal, lightDir, 100.) * 10.0;
                // scatteredLight += spec + dif;
            }
            float spec = specular(rayDir, normal, lightDir, 32.) * 1.0;
            scatteredLight += ((spec * float(enable_specular)) + (dif * float(enable_diffuse))) * refractionLoss * transmittance;
        }

        if (m_dist > maxDist /*|| m_dist < epsilon*/) 
            break;

        t += max(abs(m_dist), epsilon);
    }
    // return vec4(pos, m_dist);

    // scatteredLight += getGlow(1.0-dot(rayDir, lightDir), 0.00015, .5);
    scatteredLight += getGlow(1.0-dot(originalRayDir, lightDir), 0.00015, .5);

    return vec3(pos);
}

void main(){
    // vec2 uv = (gl_FragCoord.xy * 2.0 - u_resolution.xy) / u_resolution.y; // [-1; 1]
    // vec2 mx = (u_mouse.xy * 2.0 - u_resolution.xy) / u_resolution.y;

    vec3 rayOrigin = cameraPos;
    vec3 rayDir = normalize(fragPos - cameraPos);

    // mx*=4.0;
    // rayOrigin.yz *= rot2D(mx.y);
    // rayDir.yz *= rot2D(mx.y);

    // rayOrigin.xz *= rot2D(mx.x);
    // rayDir.xz *= rot2D(mx.x);

    vec3 scatteredLight = vec3(0.0);
    vec3 transmittance = vec3(1.0);
    vec3 result = raymarch(rayOrigin, rayDir, transmittance, scatteredLight);

    // gl_FragColor = vec4(result, 1.0);
    // return;

    vec3 background = vec3(.5);
    float mu = dot(rayDir, lightDir);
    background += getGlow(1.0-mu, 0.00015, .5);

    vec4 color = vec4(background, 0.0);
    if (transmittance.r < 1.0){
        color.xyz = transmittance + (scatteredLight);
        color.w = 0.95;
    }

    outColor = color;
}
