@group(0) @binding(0) var raymarch_depth : texture_2d<f32>; // R32Float stored as normalized depth [0..1]

struct VSOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>
};

@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> VSOut {
    var corners = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 3.0, -1.0),
        vec2<f32>(-1.0,  3.0),
    );
    var uv = (corners[idx] + 1.0) * 0.5;
    var out: VSOut;
    out.pos = vec4<f32>(corners[idx].x, corners[idx].y, 0.0, 1.0);
    out.uv = uv;
    return out;
}

@fragment
fn fs_main(in: VSOut) -> @builtin(frag_depth) f32 {
    // sample the raymarch depth stored in [0..1]
    let d = textureLoad(raymarch_depth, vec2<i32>(in.uv * vec2<f32>(textureDimensions(raymarch_depth))), 0).x;
    // if stored as NDC depth in [0..1], just return it.
    // If you stored linear view-space depth, you must convert it to NDC depth here using projection params.
    return d; // write into depth attachment
}