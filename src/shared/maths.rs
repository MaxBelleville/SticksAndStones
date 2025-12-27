use bevy::math::*;

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
