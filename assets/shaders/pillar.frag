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

const float maxDist = 100.;
const float epsilon = 0.0001;
const vec4 bgColor = vec4(0.14, 0.59, 0.73, 1.0);
const int steps = 2000;
vec3 lightDir = normalize(ubo.light_pos.xyz);
const vec3 lightColor = vec3(1.0,0.9,0.8);
const vec3 ambientColor = vec3(0.19, 0.28, 0.37);

float sdfSphere(vec3 pos, vec3 center, float s)
{
  return length(pos - center) - s;
}

float sdSphere( vec3 p, float s )
{
  return length(p)-s;
}

float opSmoothUnion( float d1, float d2, float k )
{
    float h = clamp( 0.5 + 0.5*(d2-d1)/k, 0.0, 1.0 );
    return mix( d2, d1, h ) - k*h*(1.0-h);
}

float sdTorus( vec3 p, vec2 t )
{
  vec2 q = vec2(length(p.xz)-t.x,p.y);
  return length(q)-t.y;
}

float sdVerticalCapsule( vec3 p, float h, float r )
{
  p.y -= clamp( p.y, 0.0, h );
  return length( p ) - r;
}

float sdCappedCylinder( vec3 p, float h, float r )
{
  vec2 d = abs(vec2(length(p.xz),p.y)) - vec2(r,h);
  return min(max(d.x,d.y),0.0) + length(max(d,0.0));
}

float sdBox( vec3 p, vec3 b )
{
  vec3 q = abs(p) - b;
  return length(max(q,0.0)) + min(max(q.x,max(q.y,q.z)),0.0);
}

float sdRoundBox( vec3 p, vec3 b, float r )
{
  vec3 q = abs(p) - b + r;
  return length(max(q,0.0)) + min(max(q.x,max(q.y,q.z)),0.0) - r;
}

float sdBoxFrame( vec3 p, vec3 b, float e )
{
       p = abs(p  )-b;
  vec3 q = abs(p+e)-e;
  return min(min(
      length(max(vec3(p.x,q.y,q.z),0.0))+min(max(p.x,max(q.y,q.z)),0.0),
      length(max(vec3(q.x,p.y,q.z),0.0))+min(max(q.x,max(p.y,q.z)),0.0)),
      length(max(vec3(q.x,q.y,p.z),0.0))+min(max(q.x,max(q.y,p.z)),0.0));
}

float sdRoundBoxFrame( vec3 p, vec3 b, float e, float r)
{
       p = abs(p  )-b;
  vec3 q = abs(p+e)-e;
  return min(min(
      length(max(vec3(p.x,q.y,q.z),0.0))+min(max(p.x,max(q.y,q.z)),0.0)-r,
      length(max(vec3(q.x,p.y,q.z),0.0))+min(max(q.x,max(p.y,q.z)),0.0)-r),
      length(max(vec3(q.x,q.y,p.z),0.0))+min(max(q.x,max(q.y,p.z)),0.0)-r);
}

float sdfMap(vec3 pos)
{
    // return sdBox(pos, vec3(1.0));
    
    float ground = pos.y;
    float d;
    pos.y += 1.05;
    vec3 p = pos;

    p = pos - vec3(+0,0.1,0);
    d = sdRoundBox(p, vec3(+1,0.1,1), 0.02); // bottom
    p = pos - vec3(+0,1.9,0);
    d = min(d, sdRoundBox(p, vec3(+1,0.1,1), 0.02)); // top

    p = pos - vec3(+0,1.0,0);
    d = opSmoothUnion(d, sdRoundBoxFrame(p, vec3(+0.8,0.85,0.8), 0.2, 0.02), 0.1);
    p = pos - vec3(+0,1.0,0);
    d = min(d, sdRoundBoxFrame(p, vec3(+0.7,0.75,0.7), 0.2, 0.02));
    p = pos - vec3(+0,1.,0);
    d = min(d, sdBox(p, vec3(+0.6,0.85,0.6)));

    return d;
}

vec3 calculateNormal(vec3 pos)
{
    vec2 e = vec2(1.0,-1.0)*0.5773*0.001;
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

float calculateDiffuse(vec3 pos)
{
    float eps = 0.001;
    return clamp((sdfMap(pos+eps*-lightDir)-sdfMap(pos))/eps,0.0,1.0);
}

struct RmResult {
    vec3 pos;
    vec3 prevPos;
    float t;
    float m_dist;
};

RmResult raymarch(vec3 rayOrigin, vec3 rayDir){
    RmResult d;
    d.m_dist = maxDist;
    d.t = 0.0;
    d.pos = rayOrigin;

    for (int i = 0; i < steps; i++){
      d.prevPos = d.pos;
        d.pos = rayOrigin + rayDir * d.t;
        d.m_dist = sdfMap(d.pos);

        if (d.m_dist > maxDist || d.m_dist < epsilon) break;

        d.t += d.m_dist;
    }
    return d;
}

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

float ambientOcclusion(vec3 p, vec3 n) {
    float occlusion = 0.0;
    float stepSize = 0.1; // Sample step distance
    const int numSamples = 5; // Number of AO samples
    
    for (int i = 1; i <= numSamples; i++) {
        float dist = float(i) * stepSize;
        float d = sdfMap(p + n * dist); // SDF evaluation
        occlusion += max(0.0, stepSize - d);
    }
    
    return 1.0 - (occlusion / float(numSamples));
}

void main(){

    vec3 rayOrigin = cameraPos;
    vec3 rayDir = normalize(fragPos - cameraPos);
    // vec3 color = bgColor.xyz + getGlow(1.0-dot(rayDir, lightDir), 0.00015, .5);

    vec3 background = vec3(.5);
    // float mu = dot(rayDir, lightDir);
    // background += getGlow(1.0-mu, 0.00015, .5);

    vec4 color = vec4(0);

    RmResult data = raymarch(rayOrigin, rayDir);
    float shadow = 1.0;

    if(data.m_dist <= epsilon){
        vec3 normal = calculateNormal(data.pos);
        // float diff = calculateDiffuse(data.pos);
        float diff = diffuse(normal, lightDir);
        float spec = specular(rayDir, normal, lightDir, 320.) * 0.1;

        RmResult shadowData = raymarch(data.prevPos, lightDir);
        if (shadowData.m_dist <= epsilon)
            shadow = 0.8;
        float SAO = ambientOcclusion(data.prevPos, normal);
        // SAO = 1.0;
        
        color.xyz = (lightColor * shadow * SAO * SAO * SAO * (diff + spec)) + ambientColor * SAO;
        color.w = 1.0;
        // color = vec3(SAO);
        // if(data.pos.y <= 0.01){
        //   color = (vec3(0.13, 0.9, 0.23) * shadow * SAO * (diff + spec));
        // }
    }

    outColor = color;
}