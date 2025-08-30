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
const MAX_STEPS: u32 = 1024u;
const EPSILON: f32 = 1e-5;
const MAX_DIST: f32 = 2000.0;

@group(0) @binding(0) var albedo: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(1) var normal: texture_storage_2d<rgba16float, write>;
@group(0) @binding(2) var ray_depth: texture_storage_2d<r32float, write>;

@group(1) @binding(0) var bevy_depth_tex: texture_depth_2d;
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
    is_hit: bool,
    t: f32        // ray distance (important for ordering / depth buffer)
};


// Optionally, keep the raw samples that caused the crossing
// struct Crossing {
//    p0: vec3<f32>, // world-space point before crossing
//    p1: vec3<f32>, // world-space point after crossing
//    f0: f32,       // occupancy(p0) value
//    f1: f32,       // occupancy(p1) value
//}

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
    // integer voxel coords relative to root_min (signed!)
    let iv = vec3<i32>(
        i32(floor(local.x)),
        i32(floor(local.y)),
        i32(floor(local.z))
    );

    let shift: u32 = u32(scale_exp);

    // convert to unsigned only for shifting/masking
    let iv_u = vec3<u32>(iv);
    let cell = (iv_u >> vec3<u32>(shift)) & vec3<u32>(3u);
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

fn svt_child_mask(mask: vec2<u32>, child_idx: u32) -> u32 {
    if (child_idx < 32u) {
        return (mask.x >> child_idx) & 1u;
    }
    return (mask.y >> (child_idx - 32u)) & 1u;
}

fn svt_prior_bits(mask: vec2<u32>, child_idx: u32) -> u32 {
    if (child_idx < 32u) {
        return countOneBits(mask.x & ((1u << child_idx) - 1u));
    }
    return countOneBits(mask.x) + countOneBits(mask.y & ((1u << (child_idx - 32u)) - 1u));
}

fn svt_has_children(mask: vec2<u32>) -> bool {
    let mask_nonempty = (mask.x | mask.y) != 0u;
    let mask_not_full = !(mask.x == 0xFFFFFFFFu && mask.y == 0xFFFFFFFFu);
    return mask_nonempty && mask_not_full;
}

fn svt_is_leaf(material: u32) -> bool {
    return (material & 0x80000000u) == 0u;
}

fn svt_material_index(material: u32) -> u32 {
    return material & 0xFFu;
}


// Query whether a voxel at world-space integer coordinates `p` is solid.
fn lookup_voxel(p: vec3<i32>) -> bool {
    // world voxel coords relative to root
    let local = vec3<f32>(p) - push.root_min;

    // quick reject: out of root bounds
    if (any(local < vec3<f32>(0.0)) || any(local >= vec3<f32>(push.root_size))) {
        return false;
    }

    // compute root scale exp (same as raymarch_svt)
    let root_log = i32(floor(log2(push.root_size)));
    var cur_scale: i32 = root_log - 2;

    // descend from root
    var node_idx: u32 = 0u;
    var node = load_gpu_node(node_idx);

    loop {
        // which 4×4×4 cell within this node?
        let cell_idx: u32 = get_node_cell_index(local, cur_scale);

        // does this child exist?
        if (svt_child_mask(node.mask, cell_idx) == 0u) {
            return false; // empty
        }

        // map to child node index
        let prior = svt_prior_bits(node.mask, cell_idx);
        let child_idx: u32 = node.first_child_idx + prior;
        let child_node = load_gpu_node(child_idx);

        node = child_node;
        cur_scale = cur_scale - 2;

        if (svt_is_leaf(child_node.material)) {
            if (!svt_has_children(child_node.mask)) {
                return true; // solid uniform block
            } else {
                let mixed_idx = get_node_cell_index(local, cur_scale);
                return svt_child_mask(child_node.mask, mixed_idx) == 1u;
            }
        }

        if (cur_scale < 0) {
            return true; // below voxel resolution = solid
        }
    }

    return false;
}

fn occupancy(p: vec3<i32>) -> f32 {
    // 1.0 = solid, -1.0 = air
    let hit = lookup_voxel(p); // descend SVO for that voxel
    return select(-1.0, 1.0, hit);
}

fn gradient(p: vec3<i32>) -> vec3<f32> {
    return normalize(vec3<f32>(
        occupancy(p + vec3(1,0,0)) - occupancy(p + vec3(-1,0,0)),
        occupancy(p + vec3(0,1,0)) - occupancy(p + vec3(0,-1,0)),
        occupancy(p + vec3(0,0,1)) - occupancy(p + vec3(0,0,-1))
    ));
}

fn raymarch_svt(ray_o: vec3<f32>, ray_d: vec3<f32>) -> Hit {
    let bmin = push.root_min;
    let bmax = bmin + vec3<f32>(push.root_size);

    let tb = intersect_aabb(ray_o, ray_d, bmin, bmax);
    if (tb.x > tb.y || tb.y < 0.0) {
        return Hit(vec3<f32>(0.0), vec3<f32>(0.0), 0u, false, 0.0);
    }

    // start inside bounding box
    var t: f32 = max(tb.x, 0.0) + EPSILON;
    var pos: vec3<f32> = ray_o + ray_d * t;

    // Track previous sample (init to current)
    var prev_pos: vec3<f32> = pos;
    var prev_occ: f32 = occupancy(vec3<i32>(floor(pos)));

    // DDA stepping: track which voxel we are in
    var voxel = vec3<i32>(floor(pos));
    
    for (var steps: u32 = 0u; steps < MAX_STEPS; steps = steps + 1u) {
        // check voxel occupancy
        let occ: f32 = occupancy(voxel);

        // detect sign change (air -> solid)
        if (occ > 0.0 && prev_occ < 0.0) {
            // interpolate crossing point
            let alpha: f32 = prev_occ / (prev_occ - occ); // in [0,1]
            let p_cross: vec3<f32> = mix(prev_pos, pos, alpha);

            // gradient normal
            let n = gradient(vec3<i32>(floor(p_cross)));

            return Hit(p_cross, n, 1u, true, t);
        }

        // step to next voxel boundary along ray (classic 3D DDA)
        let voxel_min: vec3<f32> = vec3<f32>(voxel);
        let voxel_max: vec3<f32> = voxel_min + 1.0;
        let tin: vec2<f32> = intersect_aabb(ray_o, ray_d, voxel_min, voxel_max);

        // advance to exit point of this voxel
        if (tin.x > tin.y) {
            t = t + 0.003;
        } else {
            t = tin.y + 0.003;
        }
        if (t >= MAX_DIST) { break; }
        
        // update positions
        prev_pos = pos;
        prev_occ = occ;
        
        pos = ray_o + ray_d * t;
        voxel = vec3<i32>(floor(pos));
    }

    return Hit(vec3<f32>(0.0), vec3<f32>(0.0), 0u, false, t);
}

fn get_camera_rotation_matrix() -> mat3x3<f32> {
    return mat3x3<f32>(
        env_data.cam_rotx,
        env_data.cam_roty,
        env_data.cam_rotz
    );
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

    var alpha = 0.0;
    var color = vec3<f32>(1.0);

   if (hit.is_hit) {
        alpha = 1.0;
      
        let N = normalize(hit.normal);
        let L = normalize(-vec3<f32>(env_data.sunx,env_data.suny,env_data.sunz));
        var base_color = vec3<f32>(0.2, 0.7, 0.2); // default green

        let lambert = max(dot(N, L), 0.0);
        
        // Add a little ambient so shadows aren’t pitch black
        let ambient = 0.2;
 
        color = base_color * (ambient + lambert);
        textureStore(normal, vec2<i32>(id.xy), vec4<f32>(hit.normal, alpha));
        textureStore(ray_depth, vec2<i32>(id.xy), vec4<f32>(hit.t));
    }

    textureStore(albedo, vec2<i32>(id.xy), vec4<f32>(color, alpha));
    textureStore(normal, vec2<i32>(id.xy), vec4<f32>(0.0));
    textureStore(ray_depth, vec2<i32>(id.xy), vec4<f32>(MAX_DIST));
   
}