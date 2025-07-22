#import bevy_pbr::forward_io::VertexOutput

@group(2) @binding(0) var material_color_texture: texture_2d<f32>;
@group(2) @binding(1) var material_color_sampler: sampler;
@group(2) @binding(2) var<uniform> is_paused: f32;

fn blur(tex_coord: vec2<f32>) -> vec4<f32> {
    // Simple blur kernel for demonstration
    let offset = 1.0 / 512.0; // adjust based on your texture size
    var color = vec4<f32>(0.0);
    color += textureSample(material_color_texture, material_color_sampler, tex_coord + vec2(-offset, 0.0));
    color += textureSample(material_color_texture, material_color_sampler, tex_coord + vec2(offset, 0.0));
    color += textureSample(material_color_texture, material_color_sampler, tex_coord + vec2(0.0, -offset));
    color += textureSample(material_color_texture, material_color_sampler, tex_coord + vec2(0.0, offset));
    color *= 0.25;
    return color;
}

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let base_color = textureSample(material_color_texture, material_color_sampler, mesh.uv);
    let blurred_color = blur(mesh.uv);

    // Mix based on pause
    return mix(base_color, blurred_color, is_paused);
}