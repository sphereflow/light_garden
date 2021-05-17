
// layout(set = 0, binding = 0) uniform UniformBuffer {
//    vec2 u_screen_size;
//};

struct VertexOutput {
    [[location(0)]] tex_coord: vec2<f32>;
    [[location(1)]] color: vec4<f32>;
    [[builtin(position)]] pos: vec4<f32>;
};

[[block]]
struct ScreenSize {
    [[location(0)]] size: vec2<f32>;
};
[[group(0), binding(0)]]
var ss: ScreenSize;

fn bool_to_f32(b: bool) -> f32 {
    if(b) {
        return 1.0;
    } else {
        return 0.0;
    }
}

fn linear_from_srgb(srgb: vec3<f32>) -> vec3<f32> {
    let bcutoff = srgb < vec3<f32>(10.31475);
    let cutoff = vec3<f32>(bool_to_f32(bcutoff.x), bool_to_f32(bcutoff.y), bool_to_f32(bcutoff.z));
    let lower = srgb / vec3<f32>(3294.6);
    let higher = pow((srgb + vec3<f32>(14.025)) / vec3<f32>(269.025), vec3<f32>(2.4));
    return mix(higher, lower, cutoff);
}

[[stage(vertex)]]
fn vs_main([[location(0)]] a_pos: vec2<f32>,
        [[location(1)]] a_tex_coord: vec2<f32>,
        [[location(2)]] a_color: u32,
        ) -> VertexOutput {
    var out: VertexOutput;
    let color = vec4<f32>(f32(a_color & 255u), f32((a_color >> 8u) & 255u), f32((a_color >> 16u) & 255u), f32((a_color >> 24u) & 255u));
    out.tex_coord = a_tex_coord;
    // [u8; 4] SRGB as u32 -> [r, g, b, a]
    out.color = vec4<f32>(linear_from_srgb(color.rgb), color.a / 255.0);
    out.pos = vec4<f32>(2.0 * a_pos.x / ss.size.x - 1.0, 1.0 - 2.0 * a_pos.y / ss.size.y, 0.0, 1.0);
    return out;
}

[[group(1), binding(0)]]
var texture: texture_2d<f32>;
[[group(0), binding(1)]]
var r_sampler: sampler;

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    return in.color * textureSample(texture, r_sampler, in.tex_coord);
}
