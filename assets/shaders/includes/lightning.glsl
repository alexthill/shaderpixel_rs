// helper functions to calculate lightning

layout(set = 0, binding = 1) uniform UniformBufferObject {
    vec4 light_pos;
    vec4 options;
    float time;
} ubo;

vec3 calc_lightning(vec3 color, vec3 pos, vec3 normal) {
    vec3 to_light_dir = normalize(ubo.light_pos.xyz - pos);
    float ambient_coef = 0.4;
    float diffuse_coef = max(0.0, dot(normal, to_light_dir));
    color = color * min(2.0, ambient_coef + diffuse_coef);
    return color;
}
