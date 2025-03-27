#version 450
#extension GL_ARB_separate_shader_objects : enable
#include "includes/lightning.glsl"

// SDF Cat by ejacquem <https://www.shadertoy.com/view/wcX3WN>

#define PI 3.1415926535

layout(location = 0) in vec3 fragPos;
layout(location = 1) in vec3 fragNorm;

float time = ubo.time * ubo.options[0][0];
layout(location = 0) out vec4 outColor;

#define PAL1 vec3(0.5,0.5,0.5),vec3(0.5,0.5,0.5),vec3(1.0,1.0,1.0),vec3(0.0,0.33,0.67)

vec3 palette(float t,vec3 a,vec3 b,vec3 c,vec3 d )
{
    return a + b*cos( 6.283185*(c*t+d) );
}

vec2 u_resolution = vec2(1000.);

void main() {
    vec2 uv = fragPos.xy; // [-1; 1]

    vec2 p = fract(uv * 5.0) - 0.5;
    p *= 10.0;

    float dist = length(p);
    dist += abs(0.01
        * dot(p, vec2(0.,1.))
        * dot(p, vec2(1.,0.))
        * dot(p, (vec2(1.,1.)))
        * dot(p, normalize(vec2(1.,-1.))));
    float offset = (uv.x * 5.0) - (uv.y * 5.0);
    float c = sin(dist + time + offset);
    float t = c*c + time * 0.1 + offset * 0.5;

    vec3 color = vec3(c * palette(t, PAL1)); 

    color = 1.0 - clamp(color,0.0, 1.0);
    outColor = vec4(calc_lightning(color, fragPos, normalize(fragNorm)), 1.0);

}
