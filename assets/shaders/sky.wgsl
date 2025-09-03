#import "shaders/shared.wgsl"::{
    PI
}

const ISTEPS: u32 = 16u;
const JSTEPS: u32 = 8u;

fn ray_sphere_interact(ray_d: vec3<f32>, atmos_ray_o: vec3<f32>, sphere_radius: f32) -> vec2<f32> {
    // ray-sphere intersection that assumes
    // the sphere is centered at the origin.
    // No intersection when result.x > result.y
    let a = dot(ray_d, ray_d);
    let b = 2.0 * dot(ray_d, atmos_ray_o);
    let c = dot(atmos_ray_o, atmos_ray_o) - (sphere_radius * sphere_radius);
    let d = (b * b) - (4.0 * a * c);

    if d < 0.0 {
        return vec2<f32>(1e5, -1e5);
    } else {
        return vec2<f32>(
            (-b - sqrt(d)) / (2.0 * a),
            (-b + sqrt(d)) / (2.0 * a)
        );
    }
}
fn get_nitsha(ray_o: vec3<f32>) ->Nishita {
    return Nishita(
        ray_o+vec3<f32>(0.0,6372e3+2000.0,0.0),
        20.0,
        6371e3,
        6471e3,
        vec3<f32>(5.8e-6, 13.5e-6, 22.0e-6),
        8e3,
        1.7e-5,
        1200.0, 
        0.875);
}

fn render_nishita(ray_d: vec3<f32>,ray_o: vec3<f32>, sun_o: vec3<f32>) -> vec3<f32> {
    let nitsha = get_nitsha(ray_o);

    let atmos_ray_o = nitsha.atmos_ray_origin;
    let sun_i = nitsha.sun_intensity;
    let planet_radius = nitsha.planet_radius;
    let atmos_radius = nitsha.atmosphere_radius;
    let rayleigh_c = nitsha.rayleigh_coefficient;
    let rayleigh_scale_h = nitsha.rayleigh_scale_height;
    let mie_c = nitsha.mie_coefficient;
    let mie_scale_h = nitsha.mie_scale_height;
    let mie_d = nitsha.mie_direction;
    // Normalize the ray direction and sun position.
    let norm_ray_d = normalize(ray_d);
    let norm_sun_o = normalize(sun_o);

    // Calculate the step size of the primary ray.
    var p = ray_sphere_interact(norm_ray_d, atmos_ray_o, atmos_radius);
    if p.x > p.y { return vec3<f32>(0f); }
    p.y = min(p.y, ray_sphere_interact(norm_ray_d, atmos_ray_o, planet_radius).x);
    let i_step_size = (p.y - p.x) / f32(ISTEPS);

    // Initialize the primary ray depth.
    var i_depth = 0.0;

    // Initialize accumulators for Rayleigh and Mie scattering.
    var total_rlh = vec3<f32>(0f);
    var total_mie = vec3<f32>(0f);

    // Initialize optical depth accumulators for the primary ray.
    var i_od_rlh = 0f;
    var i_od_mie = 0f;

    // Calculate the Rayleigh and Mie phases.
    let mu = dot(norm_ray_d, norm_sun_o);
    let mumu = mu * mu;
    let gg = mie_d * mie_d;
    let p_rlh = 3.0 / (16.0 * PI) * (1.0 + mumu);
    let p_mie = 3.0 / (8.0 * PI) * ((1.0 - gg) * (mumu + 1.0)) / (pow(1.0 + gg - 2.0 * mu * mie_d, 1.5) * (2.0 + gg));

    // Sample the primary ray.
    for (var i = 0u; i < ISTEPS; i++) {
        // Calculate the primary ray sample position.
        let i_pos = atmos_ray_o + norm_ray_d * (i_depth + i_step_size * 0.5);

        // Calculate the height of the sample.
        let i_height = length(i_pos) - planet_radius;

        // Calculate the optical depth of the Rayleigh and Mie scattering for this step.
        let od_step_rlh = exp(-i_height / rayleigh_scale_h) * i_step_size;
        let od_step_mie = exp(-i_height / mie_scale_h) * i_step_size;

        // Accumulate optical depth.
        i_od_rlh += od_step_rlh;
        i_od_mie += od_step_mie;

        // Calculate the step size of the secondary ray.
        let j_step_size = ray_sphere_interact(norm_sun_o, i_pos, atmos_radius).y / f32(JSTEPS);

        // Initialize the secondary ray depth.
        var j_depth = 0f;

        // Initialize optical depth accumulators for the secondary ray.
        var j_od_rlh = 0f;
        var j_od_mie = 0f;

        // Sample the secondary ray.
        for (var j = 0u; j < JSTEPS; j++) {

            // Calculate the secondary ray sample position.
            let j_pos = i_pos + norm_sun_o * (j_depth + j_step_size * 0.5);

            // Calculate the height of the sample.
            let j_height = length(j_pos) - planet_radius;

            // Accumulate the optical depth.
            j_od_rlh += exp(-j_height / rayleigh_scale_h) * j_step_size;
            j_od_mie += exp(-j_height / mie_scale_h) * j_step_size;

            // Increment the secondary ray depth.
            j_depth += j_step_size;
        }

        // Calculate attenuation.
        let attn = exp(-(mie_c * (i_od_mie + j_od_mie) + rayleigh_c * (i_od_rlh + j_od_rlh)));

        // Accumulate scattering.
        total_rlh += od_step_rlh * attn;
        total_mie += od_step_mie * attn;

        // Increment the primary ray depth.
        i_depth += i_step_size;
    }

    // Calculate and return the final color.
    return sun_i * (p_rlh * rayleigh_c * total_rlh + p_mie * mie_c * total_mie);
}


struct Nishita {
    atmos_ray_origin: vec3<f32>,
    sun_intensity: f32,
    planet_radius: f32,
    atmosphere_radius: f32,
    rayleigh_coefficient: vec3<f32>,
    rayleigh_scale_height: f32,
    mie_coefficient: f32,
    mie_scale_height: f32,
    mie_direction: f32,
}