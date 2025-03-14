#version 450
#extension GL_ARB_separate_shader_objects : enable

layout(location = 0) in vec3 position;

layout(binding = 0) uniform UniformBufferObject {
    mat4 model;
    mat4 view;
    mat4 proj;
} ubo;

layout(location = 0) out vec3 fragPos;
layout(location = 1) out vec3 fragNorm;

void main() {
    fragPos = position;

    mat3 norm_matrix = transpose(inverse(mat3(ubo.model)));
    fragNorm = normalize(norm_matrix * vec3(0.0, 0.0, -1.0));

    mat4 mvp = ubo.proj * ubo.view * ubo.model;
    gl_Position = mvp * vec4(position, 1.0);
    gl_Position.y = -gl_Position.y;
}
