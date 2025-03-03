#version 450
#extension GL_ARB_separate_shader_objects : enable

layout(location = 0) in vec3 position;

layout(binding = 0) uniform UniformBufferObject {
    mat4 model;
    mat4 view;
    mat4 proj;
} ubo;

layout(location = 0) out vec3 fragPos;

void main() {
    fragPos = position;

    mat4 mvp = ubo.proj * ubo.view * ubo.model;
    gl_Position = mvp * vec4(position, 1.0);
    gl_Position.y = -gl_Position.y;
}
