#version 300 es

layout (location = 0) in vec3 position;
layout (location = 1) in vec3 normal;
layout (location = 2) in vec4 color;

uniform mat4 u_model;
uniform mat4 u_view;
uniform mat4 u_projection;

uniform vec3 u_meshTranslation;
uniform float u_meshScale;

out vec3 fragPosition;
out vec3 fragNormal;
out vec4 fragColor;

void main() {
    vec3 real_position = (u_meshScale * position) + (u_meshTranslation * vec3(1.0, -1.0, 1.0));
    gl_Position = u_projection * u_view * u_model * vec4(real_position, 1.0);

    fragPosition = vec3(u_model * vec4(real_position, 1.0));
    fragNormal = mat3(transpose(inverse(u_model))) * normal;

    fragColor = color;
}
