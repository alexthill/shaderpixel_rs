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

// stolen from <https://www.shadertoy.com/view/3s3GDn>
float getGlow(float dist, float radius, float intensity){
    return max(0.0, pow(radius/max(dist, 1e-5), intensity));
}

void main() {
    vec3 dir = normalize(fragPos - cameraPos);
    vec3 sun_dir = normalize(vec3(1.0, 1.0, 1.0));
    float sun_angle = dot(dir, sun_dir);

    outColor = vec4(dir * 0.4 + 0.4, 1.0) + getGlow(1 - sun_angle, 0.00015, 0.5);
}
