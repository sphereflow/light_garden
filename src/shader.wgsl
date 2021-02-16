[[location(0)]]
var<in> in_pos: vec2<f32>;
[[location(1)]]
var<in> in_color_vs: vec4<f32>;
[[location(0)]]
var<out> in_color_fs: vec4<f32>;
[[builtin(position)]]
var<out> out_pos: vec4<f32>;

[[block]]
struct Transform {
  transform: mat4x4<f32>;
};

[[group(0), binding(0)]]
var u_transform: Transform;

[[stage(vertex)]]
fn vs_main() {
    out_pos = u_transform.transform * vec4<f32>(in_pos, 0.0, 1.0);
    in_color_fs = in_color_vs;
}

[[location(0)]]
var<in> in_color_fs: vec4<f32>;
[[location(0)]]
var<out> out_color_fs: vec4<f32>;

[[stage(fragment)]]
fn fs_main() {
    out_color_fs = in_color_fs;
}
