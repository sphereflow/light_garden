#version 450

layout(location = 0) in vec4 v_Color;
layout(location = 1) in vec2 v_tex_coord;
layout(location = 0) out vec4 f_Color;

layout(set = 1, binding = 0) uniform texture2D t_texture;
layout(set = 0, binding = 1) uniform sampler s_texture;

void main() {
    f_Color = v_Color + texture(sampler2D(t_texture, s_texture), v_tex_coord);
}
