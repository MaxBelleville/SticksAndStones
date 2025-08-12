struct EnviromentData {
    cam_rotx: vec3<f32>,
    sunx: f32,
    cam_roty: vec3<f32>,
    suny: f32,
    cam_rotz: vec3<f32>,
    sunz: f32,
    cam_pos: vec3<f32>,
    time: f32,
};

struct PushConstants {
    screen_size: vec2<f32>,
    fov_y_deg: f32,
    root_size: f32,
    root_min: vec3<f32>,
    mode: f32,
};
const NOT_A_LEAF_FLAG: u32 = 2147483648u;
const MAX_STEPS: u32 = 256u;
const EPSILON: f32 = 1e-2;
const MAX_DIST: f32 = 1000.0;

@group(0) @binding(0) var input: texture_storage_2d<rgba8unorm, read>;
@group(0) @binding(1) var output: texture_storage_2d<rgba8unorm, write>;

@group(1) @binding(0) var depth_tex: texture_depth_2d;
@group(1) @binding(1) var<uniform> env_data: EnviromentData;
@group(1) @binding(2) var<storage, read> svt_data: array<i32>;
@group(1) @binding(3) var<uniform> push: PushConstants;

struct GpuNode {
    mask: vec2<u32>,
    first_child_idx: u32,
    material: u32,
};
struct Hit {
    pos: vec3<f32>,
    normal: vec3<f32>,
    material: u32,
    is_hit: bool
};

fn load_gpu_node(index: u32) -> GpuNode {
    let base = index * 4u;

    let mask_low: u32 = u32(svt_data[base + 0u]);
    let mask_high: u32 = u32(svt_data[base + 1u]);
    let first_child_idx: u32 = u32(svt_data[base + 2u]);
    let material: u32 = u32(svt_data[base + 3u]);

    return GpuNode(vec2<u32>(mask_low, mask_high), first_child_idx, material);
}

// Get child index (0..63) for local world-space position `local`
// `local` is in world units relative to root_min. We floor() it to voxels first.
fn get_node_cell_index(local: vec3<f32>, scale_exp: i32) -> u32 {
    // integer voxel coords relative to root_min
    let iv = vec3<u32>(u32(floor(local.x)), u32(floor(local.y)), u32(floor(local.z)));
    let shift = u32(scale_exp);
    let cell = (iv >> vec3<u32>(shift)) & vec3<u32>(3u); // take 2 bits per axis
    return cell.x | (cell.y << 2u) | (cell.z << 4u);
}

fn floor_scale(local: vec3<f32>, scale_exp: i32) -> vec3<f32> {
    if (scale_exp <= 0) {
        // voxel size 1.0
        return vec3<f32>(floor(local.x), floor(local.y), floor(local.z));
    }
    let block = pow(2.0, f32(scale_exp)); // block size in voxels
    return vec3<f32>(floor(local.x / block) * block,
                     floor(local.y / block) * block,
                     floor(local.z / block) * block);
}

fn first_leading_bit(x: u32) -> i32 {
    return i32(31u - countLeadingZeros(x));
}

fn highest_diff_exp(a: vec3<u32>, b: vec3<u32>) -> i32 {
    let diff = a ^ b;
    let merged = diff.x | diff.y | diff.z;
    return first_leading_bit(merged & 0xFFAAAAAAu); // interleaved axis bits mask
}

fn svo_child_mask(mask: vec2<u32>, child_idx: u32) -> u32 {
    if (child_idx < 32u) {
        return (mask.x >> child_idx) & 1u;
    }
    return (mask.y >> (child_idx - 32u)) & 1u;
}

fn svo_prior_bits(mask: vec2<u32>, child_idx: u32) -> u32 {
    if (child_idx < 32u) {
        return countOneBits(mask.x & ((1u << child_idx) - 1u));
    }
    return countOneBits(mask.x) + countOneBits(mask.y & ((1u << (child_idx - 32u)) - 1u));
}

fn svo_is_leaf(material: u32) -> bool {
    return (material & 0x80000000u) == 0u;
}

fn svo_material_index(material: u32) -> u32 {
    return material & 0xFFu;
}

fn get_camera_rotation_matrix() -> mat3x3<f32> {
    return mat3x3<f32>(
        env_data.cam_rotx,
        env_data.cam_roty,
        env_data.cam_rotz
    );
}

fn raymarch_svt(ray_o: vec3<f32>, ray_d: vec3<f32>) -> Hit {
    let bmin = push.root_min;
    let root_size = push.root_size;
    let bmax = bmin + vec3<f32>(root_size);

    let tb = intersect_aabb(ray_o, ray_d, bmin, bmax);
    if (tb.x > tb.y || tb.y < 0.0) {
        return Hit(vec3<f32>(0.0), vec3<f32>(0.0), 0u, false);
    }

    // initial t and sample position
    var t: f32 = max(tb.x, 0.0) + EPSILON;
    var pos: vec3<f32> = ray_o + ray_d * t;

    // compute scale_exp = log2(root_size) - 2  (top level child size = root_size / 4)
    let root_log = i32(floor(log2(root_size)));
    var scale_exp: i32 = root_log - 2;

    for (var steps: u32 = 0u; steps < MAX_STEPS; steps = steps + 1u) {
        // position relative to root_min
        let local = pos - bmin;

        // out of root?
        if (any(local < vec3<f32>(0.0)) || any(local >= vec3<f32>(root_size))) {
            break;
        }

        // Descend the SVO for the current local position
        var node_idx: u32 = 0u;
        var node = load_gpu_node(node_idx);
        var cur_scale: i32 = scale_exp;
        var hit_leaf: bool = false;
        var hit_material: u32 = 0u;
        var hit_voxel_min: vec3<f32> = vec3<f32>(0.0);

        loop {
            // which child (0..63) at this node does `local` lie in?
            let cell_idx: u32 = get_node_cell_index(local, cur_scale);

            // does this child exist?
            if (svo_child_mask(node.mask, cell_idx) == 0u) {
                // missing child => empty at this subtree
                hit_leaf = false;
                break;
            }

            // compute child index (compacted)
            let prior = svo_prior_bits(node.mask, cell_idx);
            let child_node_idx: u32 = node.first_child_idx + prior;
            let child_node = load_gpu_node(child_node_idx);

            // if child encodes a leaf material -> hit
            if (svo_is_leaf(child_node.material)) {
                hit_leaf = true;
                hit_material = svo_material_index(child_node.material);
                // compute voxel-aligned min for the leaf at this resolution (voxel size = 1)
                hit_voxel_min = bmin + floor(local); // world-space voxel min
                break;
            }

            // descend into child node
            node_idx = child_node_idx;
            node = child_node;
            cur_scale = cur_scale - 2;

            // if we've reached below voxel resolution, treat as leaf
            if (cur_scale < 0) {
                hit_leaf = true;
                // if child_node.material wasn't leaf-encoded, still treat this as filled
                hit_material = svo_material_index(child_node.material);
                hit_voxel_min = bmin + floor(local);
                break;
            }
        } // end loop descend

        if (hit_leaf) {
            let hit_pos = hit_voxel_min + vec3<f32>(0.5); // center of voxel as hit pos
            let normal = -normalize(ray_d);
            return Hit(hit_pos, normal, hit_material, true);
        }

        // No leaf here -> step to the exit of the integer voxel we are currently in
        let voxel_min_world = bmin + vec3<f32>(floor(local));
        let voxel_max_world = voxel_min_world + vec3<f32>(1.0);

        let tin = intersect_aabb(ray_o, ray_d, voxel_min_world, voxel_max_world);
        if (tin.x > tin.y) {
            // numerical weirdness: advance a bit
            t = t + EPSILON;
        } else {
            t = tin.y + EPSILON;
        }
        if (t >= MAX_DIST) { break; }
        pos = ray_o + ray_d * t;
    }

    return Hit(vec3<f32>(0.0), vec3<f32>(0.0), 0u, false);
}

fn intersect_aabb(origin: vec3<f32>, dir: vec3<f32>, min: vec3<f32>, max: vec3<f32>) -> vec2<f32> {
    let inv_dir = 1.0 / dir;
    let t0 = (min - origin) * inv_dir;
    let t1 = (max - origin) * inv_dir;
    let tmin = max(max(min(t0.x, t1.x), min(t0.y, t1.y)), min(t0.z, t1.z));
    let tmax = min(min(max(t0.x, t1.x), max(t0.y, t1.y)), max(t0.z, t1.z));
    return vec2<f32>(tmin, tmax);
}

fn screen_uv_to_ray_dir(uv: vec2<f32>, fov_y_deg: f32, aspect: f32) -> vec3<f32> {
    let fov_y = radians(fov_y_deg);
    let y = tan(fov_y * 0.5);
    let x = y * aspect;

    // flip Y for NDC
    let screen = vec2<f32>(uv.x * 2.0 - 1.0, (1.0 - uv.y) * 2.0 - 1.0);
    let dir_camera = normalize(vec3<f32>(screen.x * x, screen.y * y, -1.0));
    
    // Adjust the ray direction based on the camera's rotation
    return normalize(get_camera_rotation_matrix() * dir_camera);
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x >= u32(push.screen_size.x) || id.y >= u32(push.screen_size.y)) {
        return;
    }
    
    let uv = ((vec2<f32>(id.xy)) / push.screen_size);
    //Reduce skew due to percision issue on fullscreen.
    let aspect = push.screen_size.x / push.screen_size.y;

   
    // Calculate the ray direction
    let dir_camera = screen_uv_to_ray_dir(uv, push.fov_y_deg, aspect);
    let ray_org = env_data.cam_pos + dir_camera*EPSILON;
    let hit = raymarch_svt(ray_org, dir_camera);

    var out_val = 0.0;
    var color = vec3<f32>(0.0);

    if (hit.is_hit) {
        out_val = 1.0;

        let t = env_data.time;
        let angle = dot(hit.pos.xz, vec2(12.0, 7.0)) + t;
        let angle2 = dot(hit.pos.xy, vec2(12.0, 7.0)) + t;
        let angle3 = dot(hit.pos.zy, vec2(12.0, 7.0)) + t;
        let r = 0.5 + 0.5 * sin(angle);
        let g = 0.5 + 0.5 * sin(angle2 + 2.0);
        let b = 0.5 + 0.5 * sin(angle3 + 4.0);
        color = vec3(r,g,b);
    }

    textureStore(output, vec2<i32>(id.xy), vec4<f32>(color, out_val));
}