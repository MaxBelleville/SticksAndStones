use bevy::math::*;


#[inline]
pub fn mask2(input: usize) -> usize {
    input & 0b11 // Only 2 bits per axis (values 0–3)
}
#[inline]
pub fn ileave3(pos: IVec3) -> usize {
    ((mask2(pos.z as usize)) | (mask2(pos.y as usize) << 2) | mask2(pos.x as usize) << 4) as usize
}

#[inline]
fn part1by2(n: u32) -> u64 {
    let mut x = n as u64 & 0x1f_ffff; // 21 bits max
    x = (x | (x << 32)) & 0x1f00_0000_00ff_ff;
    x = (x | (x << 16)) & 0x1f00_00ff_0000_ff;
    x = (x | (x << 8))  & 0x100f_00f0_0f00_f00f;
    x = (x | (x << 4))  & 0x10c3_0c30_c30c_30c3;
    x = (x | (x << 2))  & 0x1249_2492_4924_9249;
    x
}

#[inline]
pub fn ileavechunk(x: u32, y: u32, z: u32) -> usize {
    (part1by2(x) | (part1by2(y) << 1) | (part1by2(z) << 2)) as usize
}


#[inline]
pub fn child_index(morton: usize, shift: usize) -> usize {
    // bit layout at (morton >> shift) low 6 bits:
    // bit 0 = x0, bit1 = y0, bit2 = z0, bit3 = x1, bit4 = y1, bit5 = z1
    let b0 = ((morton >> shift) & 1) as usize;
    let b1 = ((morton >> (shift + 1)) & 1) as usize;
    let b2 = ((morton >> (shift + 2)) & 1) as usize;
    let b3 = ((morton >> (shift + 3)) & 1) as usize;
    let b4 = ((morton >> (shift + 4)) & 1) as usize;
    let b5 = ((morton >> (shift + 5)) & 1) as usize;

    let x = (b3 << 1) | b0; // x1x0
    let y = (b4 << 1) | b1; // y1y0
    let z = (b5 << 1) | b2; // z1z0

    (x << 4) | (y << 2) | z
}

// Extent in voxels per axis that a lod-level covers (level=0 -> 1, level=1 -> 4, ...)
#[inline]
pub fn level_extent(level: u8) -> usize {
    1 << (2 * level as usize) // 4^level
}

#[inline]
pub fn div_floor(a: i32, b: i32) -> i32 {
    let d = a / b;
    let r = a % b;
    if (r != 0) && ((r < 0) != (b < 0)) {
        d - 1
    } else {
        d
    }
}

#[inline]
pub fn div_floor_vec3(a: IVec3, b: i32) -> IVec3 {
    ivec3(div_floor(a.x,b),div_floor(a.y,b),div_floor(a.z,b))
}

fn mod_floor(a: i32, b: i32) -> i32 {
    ((a % b) + b) % b
}

#[inline]
pub fn mod_floor_vec3(a: IVec3, b: i32) -> IVec3 {
    ivec3(mod_floor(a.x,b),mod_floor(a.y,b),mod_floor(a.z,b))
}

#[inline]
pub fn log4(size: usize) -> usize {
    // assumes size is a power of 4; add checks if you want
    (size as f64).log(4.0).round() as usize
}

#[inline]
pub fn pow64(exp: u32) -> usize {
    // 64^exp
    1usize.checked_shl(6 * exp).unwrap_or(usize::MAX)
}

#[inline]
pub fn total_nodes(chunk_size: usize) -> usize {
    let d = log4(chunk_size);             // chunk_size = 4^d
    //let k = d.saturating_sub(1);            // subdivision levels until 4^3
    let top = pow64(d as u32);                   // 64^d
    (top - 1) / 63                          // sum_{i=0..k} 64^i
}

pub fn estimate_nodes(voxel_count: usize, chunk_size: usize) -> usize {
    if voxel_count == 0 {
        return 1 + 64; // you allocate root + first 64 children immediately
    }
    let d = log4(chunk_size);
    let k = d.saturating_sub(1);
    let m = pow64(k as u32);                     // max number of 4^3 leaves = 64^k

    let l_max = voxel_count.min(m);         // worst-case leaves (one voxel per 4^3 cell)
    let internal_cap = (m - 1) / 63;        // full internal nodes (root..level k-1)
    let i_max = (l_max.saturating_mul(k as usize)).min(internal_cap);

    let n_upper = l_max + i_max;
    n_upper.max(1 + 64)                     // ensure room for your initial alloc
}


#[inline]
pub fn deileave3(idx: usize,child_size: i32) -> IVec3 {
    let xi = mask2(idx >> 4) as i32;
    let yi = mask2(idx >> 2) as i32;
    let zi = mask2(idx) as i32;

    IVec3::new(xi,yi,zi)* child_size
}
#[inline]
pub fn pathtrace_to_world_pos(
    root_size: usize,
    paths: &[u8],
) -> IVec3 {
    let mut pos =  IVec3::splat(-(root_size as i32) / 2);
    let mut size = root_size as i32;

    for &path in paths {
        let i = path;
        let child_width = size / 4;
        
        pos += deileave3(i as usize,child_width);
        // godot_print!("{} {} {}", i , contree_utils::deileave3(i as usize), size);
        size = child_width;
    }

    pos
}
