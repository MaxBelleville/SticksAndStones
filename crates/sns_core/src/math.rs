use bevy_math::*;

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
pub fn hash_str(s: &str, seed: i32) -> u32 {
    let mut h = seed as u32;
    for &byte in s.as_bytes() {
        h ^= byte as u32;
        h = h.wrapping_mul(0x9E3779B9);
    }

    h = h.wrapping_mul(0xC2B2AE35);
    h
}

#[inline]
pub fn hash_id(id: u32, seed: i32) -> u32 {
    let mut h = seed as u32;

    for &byte in id.to_be_bytes().iter() {
        h ^= byte as u32;
        h = h.wrapping_mul(0x9E3779B9);
    }

    h = h.wrapping_mul(0xC2B2AE35);
    h
}

#[inline]
pub fn hash_str_vec2(s: &str, seed: i32) -> Vec2 {
    let h = hash_str(s, seed);
    Vec2::new(
        (h & 0xFFFF) as f32 / 65535.0,
        ((h >> 16) & 0xFFFF) as f32 / 65535.0,
    )
}

#[inline]
pub fn hash_id_vec2(id: u32, seed: i32) -> Vec2 {
    let h = hash_id(id, seed);
    Vec2::new(
        (h & 0xFFFF) as f32 / 65535.0,
        ((h >> 16) & 0xFFFF) as f32 / 65535.0,
    )
}

#[inline]
pub fn hash2(coord: IVec2, seed: i32) -> u32 {
    let mut h = seed as u32;
    h ^= coord.x as u32 * 0x9E3779B9;
    h ^= coord.y as u32 * 0x85EBCA6B;
    h = h.wrapping_mul(0xC2B2AE35);
    h
}

#[inline]
pub fn hash_vec2(coord: IVec2, seed: i32) -> Vec2 {
    let h = hash2(coord, seed);
    Vec2::new(
        (h & 0xFFFF) as f32 / 65535.0,
        ((h >> 16) & 0xFFFF) as f32 / 65535.0,
    )
}
#[inline]
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + t * (b - a)
}
#[inline]
pub fn smoothstep(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

#[inline]
pub fn morton_split(mut x: u32) -> u32 {
    x &= 0x000003ff;
    x = (x | (x << 16)) & 0xff0000ff;
    x = (x | (x << 8)) & 0x0300f00f;
    x = (x | (x << 4)) & 0x030c30c3;
    x = (x | (x << 2)) & 0x09249249;
    x
}

pub fn morton3(x: u32, y: u32, z: u32) -> u32 {
    morton_split(x) | (morton_split(y) << 1) | (morton_split(z) << 2)
}

pub fn next_pow2(x: usize) -> usize {
    x.next_power_of_two()
}