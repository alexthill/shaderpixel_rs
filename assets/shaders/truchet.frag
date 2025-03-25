
float time = 0.0;

const float PI2 = 6.283;

const float maxDist = 10.;
const float epsilon = 0.001;
const vec4 bgColor = vec4(1.0);
const int steps = 100;
const vec3 lightDir = normalize(vec3(1.2, 1, -1.1));
vec3 railColor = vec3(0);

#define PAL1 vec3(0.5,0.5,0.5),vec3(0.5,0.5,0.5),vec3(1.0,1.0,1.0),vec3(0.0,0.33,0.67)
#define PAL2 vec3(0.5,0.5,0.5),vec3(0.5,0.5,0.5),vec3(1.0,1.0,1.0),vec3(0.0,0.10,0.20) 
#define PAL3 vec3(0.5,0.5,0.5),vec3(0.5,0.5,0.5),vec3(1.0,1.0,1.0),vec3(0.3,0.20,0.20)
#define PAL4 vec3(0.5,0.5,0.5),vec3(0.5,0.5,0.5),vec3(1.0,1.0,0.5),vec3(0.8,0.90,0.30)
#define PAL5 vec3(0.5,0.5,0.5),vec3(0.5,0.5,0.5),vec3(1.0,0.7,0.4),vec3(0.0,0.15,0.20)
#define PAL6 vec3(0.5,0.5,0.5),vec3(0.5,0.5,0.5),vec3(2.0,1.0,0.0),vec3(0.5,0.20,0.25)
#define PAL7 vec3(0.8,0.5,0.4),vec3(0.2,0.4,0.2),vec3(2.0,1.0,1.0),vec3(0.0,0.25,0.25)

vec3 palette(float t,vec3 a,vec3 b,vec3 c,vec3 d )
{
    return a + b*cos( 6.283185*(c*t+d) );
}

float hash13(vec3 p3)
{
	p3  = fract(p3 * .1031);
    p3 += dot(p3, p3.zyx + 31.32);
    return fract((p3.x + p3.y) * p3.z);
}

float sdfSphere(vec3 pos, float s)
{
  return length(pos) - s;
}

float opSmoothUnion( float d1, float d2, float k )
{
    float h = clamp( 0.5 + 0.5*(d2-d1)/k, 0.0, 1.0 );
    return mix( d2, d1, h ) - k*h*(1.0-h);
}

mat2 rot2D(float angle)
{
    // float c = cos(angle);
    // float s = sin(angle);
    // return mat2(c, s, -s, c);
    vec2 a = sin(vec2(1.5707963, 0) + angle); 
    return mat2(a, -a.y, a.x);
}

// Mobius equation from https://www.shadertoy.com/view/XldSDs
const float toroidRadius = 0.5; // The object's disc radius.
const float polRot = 3.; // Poloidal rotations.
const float ballnb = 5.0 * 4.0;
float sdfMobius(vec3 p, float a){
    p.xz *= rot2D(a);
    p.x -= toroidRadius;
    p.xy *= rot2D(a*polRot + time);

    p = abs(abs(p) - .06);
    return sdfSphere(p, .061);
}

float sdfSphereTorus(vec3 p, float a){
    float ia = (floor(ballnb*a/PI2) + .5)/ballnb*PI2; 

    p.xz *= rot2D(ia);
    p.x -= toroidRadius;

    return sdfSphere(abs(p), 0.05);
}

vec2 objId;

float sdRotatingTorus(vec3 pos){
    float r = 1.0;
    vec3 p = pos;
    float sdfS, sdfT;

    p.xz *= rot2D(radians(time * 10. * r));

    float a = atan(p.z, p.x);
    sdfT = sdfMobius(p, a);
    sdfS = sdfSphereTorus(p, a);

    objId[0] = min(sdfS, objId[0]);
    objId[1] = min(sdfT, objId[1]);

    return opSmoothUnion(sdfS, sdfT, 0.00);
}

const mat2 rot90 = mat2(0, 1, -1, 0);
float sdfMap(vec3 pos)
{
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
    return palette(trilinearInterpolation(pos + time * 0.2) * 2.0, PAL1);
}

vec4 raymarch(vec3 rayOrigin, vec3 rayDir){
    float m_dist = maxDist;
    float t = 0.0; // total dist
    vec3 pos = vec3(0);
    vec3 startPos = rayOrigin;
    int j = 0;

    for (int i = 0; i < steps; i++){
        pos = startPos + rayDir * t;

        m_dist = sdfMap(pos);

        // if(m_dist < -epsilon){ // Error detection
        //     gl_FragColor = vec4(vec3(abs(sin(time * 3.))),1);
        //     return vec4(pos, 0.);
        // }

        if (m_dist < epsilon || m_dist > maxDist) 
            break;

        t += m_dist;
        j++;
    }
    return vec4(pos, float(j));
}

vec3 sdfColor(vec3 pos){
    if (objId[0] < objId[1])
        return get3dColorGradient(pos);
    // else
    //     return railColor;
    vec3 c, c1, c2;
    c1 = get3dColorGradient(pos);
    c2 = railColor;
    float d = abs(objId[0] - objId[1]);
    c = max(vec3(0),mix(c1, c2, d * 22.));
    return c;
}

//return the color of the scene
vec3 truchetRaymarching(vec3 rayOrigin, vec3 rayDir, inout float depth, float t){
    time = t;
    vec4 result = raymarch(rayOrigin, rayDir);
    vec3 pos = result.xyz;

    float dist = distance(pos, rayOrigin);
    depth = 1.0 - (dist / (maxDist));

    vec3 sphereColor = sdfColor(pos);

    vec3 color = sphereColor * depth;
    return color;
}

// void main(){
//     vec2 uv = (gl_FragCoord.xy * 2.0 - u_resolution.xy) / u_resolution.y; // [-1; 1]
//     vec2 mx = (u_mouse.xy * 2.0 - u_resolution.xy) / u_resolution.y;

//     vec3 rayOrigin = vec3(0,0,0);
//     vec3 rayDir = normalize(vec3(uv, 0.99));

//     mx *= 4.0;
//     // rayOrigin.yz *= rot2D(mx.y);
//     rayDir.yz *= rot2D(mx.y);
//     // rayOrigin.xz *= rot2D(mx.x);
//     rayDir.xz *= rot2D(mx.x);

//     gl_FragColor = vec4(truchetRaymarching(rayOrigin, rayDir), 1.0);
// }