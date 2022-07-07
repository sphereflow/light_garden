struct VertexOutput {
    @location(0) tex_coord: vec2<f32>,
    @builtin(position) pos: vec4<f32>,
};

@vertex
fn vs_main(
    @location(0) pos: vec2<f32>,
    @location(2) tex_coord: vec2<f32>,
        ) -> VertexOutput {
    var out: VertexOutput;
    out.pos = vec4<f32>(pos, 0.0, 1.0);
    out.tex_coord = tex_coord;
    return out;
}

@group(0)
@binding(0)
var texture: texture_2d<f32>;

@group(0)
@binding(1)
var r_sampler: sampler;

@fragment
fn fs_main(@location(0) tex_coord: vec2<f32>) -> @location(0) vec4<f32> {
    return textureSample(texture, r_sampler, tex_coord);
}
