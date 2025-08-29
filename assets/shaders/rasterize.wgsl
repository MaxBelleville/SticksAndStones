@group(0) @binding(0) var ray_depth_in: texture_storage_2d<r32float, read>;
@group(0) @binding(1) var albedo: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(2) var normal: texture_storage_2d<rgba16float, write>;
@group(0) @binding(3) var ray_depth_out: texture_storage_2d<r32float, write>;

