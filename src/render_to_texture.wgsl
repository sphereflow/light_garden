[[location(0)]]
var<in> in_pos: vec2<f32>;
[[location(2)]]
var<in> in_tex_coord_vs: vec2<f32>;
[[location(0)]]
var<out> out_tex_coord: vec2<f32>;
[[builtin(position)]]
var<out> out_pos: vec4<f32>;

[[stage(vertex)]]
fn vs_main() {
    out_pos = vec4<f32>(in_pos, 0.0, 1.0);
    out_tex_coord = in_tex_coord_vs;
}

[[location(0)]]
var<in> in_tex_coord_fs: vec2<f32>;
[[location(0)]]
var<out> out_color_fs: vec4<f32>;

[[group(0), binding(0)]]
var r_color: texture_2d<f32>;
[[group(0), binding(1)]]
var r_sampler: sampler;

[[stage(fragment)]]
fn fs_main() {
    out_color_fs = textureSample(r_color, r_sampler, in_tex_coord_fs);
}
