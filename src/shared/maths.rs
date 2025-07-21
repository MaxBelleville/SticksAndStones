use bevy::math::*;

#[inline]
pub fn get_noise_seed(seed: &[u8;32]) -> Vec3 {
    let part1 = &seed[0..11];
    let part2 = &seed[11..22]; 
    let part3 = &seed[22..32]; 

    let sum1 = part1.iter().map(|&b| b as u32).sum::<u32>() as f32;
    let sum2 = part2.iter().map(|&b| b as u32).sum::<u32>() as f32;
    let sum3 = part3.iter().map(|&b| b as u32).sum::<u32>() as f32;
    Vec3::new(sum1,sum2,sum3)
}

#[inline]
pub fn screen_size_quad(dist: f32, fov: f32, dim: Vec4,) -> Vec2 {
    let aspect_ratio = (dim.z as f32) /  (dim.w as f32);
    let plane_height = 2.0 * dist * (fov / 2.0).tan();
    let plane_width = plane_height * aspect_ratio;
    //Half size is not perfect so mild adjustment are needed to fit screen.
    Vec2::new(plane_width / 1.995, plane_height / 2.00)

}


#[inline]
pub fn ileave_two0(input: usize) -> usize {
    input & 0b11 // Only 2 bits per axis (values 0–3)
}
#[inline]
pub fn ileave3(pos: IVec3) -> usize {
    ((ileave_two0(pos.z as usize) << 4) | (ileave_two0(pos.y as usize) << 2) | ileave_two0(pos.x as usize)) as usize
}
#[inline]
pub fn deileave3(idx: usize) -> IVec3 {
    let xi = ileave_two0(idx) as i32;
    let yi = ileave_two0(idx >> 2) as i32;
    let zi = ileave_two0(idx >> 4) as i32;

    IVec3::new(xi,yi,zi)
}
#[inline]
pub fn pathtrace_to_world_pos(
    root_size: usize,
    path: &[Option<u8>],
) -> IVec3 {
    let mut pos =  IVec3::splat(-(root_size as i32) / 2);
    let mut size = root_size as i32;

    for &optional in path {
        let i = optional.unwrap_or(0);
        let child_width = size / 4;
        
        pos += deileave3(i as usize) * child_width;
        // godot_print!("{} {} {}", i , contree_utils::deileave3(i as usize), size);
        size = child_width;
    }

    pos
}
