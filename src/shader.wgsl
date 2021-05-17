struct VertexOutput {
    [[location(0)]] in_color_fs: vec4<f32>;
    [[builtin(position)]] out_pos: vec4<f32>;
};

[[block]]
struct Transform {
    transform: mat4x4<f32>;
};

[[group(0), binding(0)]]
var u_transform: Transform;

[[stage(vertex)]]
fn vs_main(
        [[location(0)]] in_pos: vec2<f32>,
        [[location(1)]] in_color_vs: vec4<f32>,
        ) -> VertexOutput {
    var out: VertexOutput;
    out.out_pos = u_transform.transform * vec4<f32>(in_pos, 0.0, 1.0);
    out.in_color_fs = in_color_vs;
    return out;
}

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    return in.in_color_fs;
}
