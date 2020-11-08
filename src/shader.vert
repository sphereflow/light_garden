#version 450

layout(location = 0) in vec2 a_Pos;
layout(location = 1) in vec4 a_Color;
layout(location = 0) out vec4 f_Color;

layout(set = 0, binding = 0) uniform Locals {
    mat4 u_Transform;
};

void main() {
    gl_Position = u_Transform * vec4(a_Pos, 0.0, 1.0);
    f_Color = a_Color;
}
