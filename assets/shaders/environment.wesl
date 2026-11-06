
#import "shaders/shared.wesl"::{
    env
}

fn sky_color(ray_dir: vec3<f32>, h: f32) -> vec3<f32> {

    // horizon → mid
    let lower = smoothstep(
        0.0,
        env.constants.sky_mid_height,
        h
    );

    let horizon_mid = mix(
        env.dynamic.horizon_color,
        env.dynamic.mid_color,
        lower
    );

    // mid → zenith
    let upper = smoothstep(
        env.constants.sky_mid_height,
        1.0,
        h
    );

    let sky = mix(
        horizon_mid,
        env.dynamic.zenith_color,
        upper
    );

    return pow(sky, vec3<f32>(env.constants.gradient_exponent));
}

fn sun_disk(ray_dir: vec3<f32>) -> vec3<f32> {
    let sun_dir = normalize(env.dynamic.sun_direction);
    let sun_dot = dot(ray_dir, sun_dir);

    let disk = smoothstep(
        1.0 - env.constants.sun_size,
        1.0 - env.constants.sun_size + env.constants.sun_softness,
        sun_dot
    );

    let core = pow(disk, 3.0);

    return core * env.dynamic.sun_color * env.dynamic.sun_intensity * 6.0;
}

fn alpha_moon_disk(ray_dir: vec3<f32>) -> vec3<f32> {
    let moon_dir = normalize(env.dynamic.alpha_moon_direction);
    let moon_dot = dot(ray_dir, moon_dir);

    let disk = smoothstep(
        1.0 - env.constants.alpha_moon_size,
        1.0 - env.constants.alpha_moon_size + env.constants.sun_softness,
        moon_dot
    );

    return disk * env.dynamic.alpha_moon_color * env.dynamic.alpha_moon_intensity;
}

fn alpha_moon_glow(ray_dir: vec3<f32>) -> vec3<f32> {
    let mu = env_mu(ray_dir, env.dynamic.alpha_moon_direction);
    let glow = pow(mu, 16.0);

    return glow * env.dynamic.alpha_moon_color * 0.075;
}

fn beta_moon_disk(ray_dir: vec3<f32>) -> vec3<f32> {
    let moon_dir = normalize(env.dynamic.beta_moon_direction);
    let moon_dot = dot(ray_dir, moon_dir);

    let disk = smoothstep(
        1.0 - env.constants.beta_moon_size,
        1.0 - env.constants.beta_moon_size + env.constants.sun_softness,
        moon_dot
    );

    return disk * env.dynamic.beta_moon_color * env.dynamic.beta_moon_intensity;
}

fn beta_moon_glow(ray_dir: vec3<f32>) -> vec3<f32> {
   let mu = env_mu(ray_dir, env.dynamic.beta_moon_direction);
   let glow = pow(mu, 16.0);

   return glow * env.dynamic.beta_moon_color * 0.05;
}

fn sun_glow(ray_dir: vec3<f32>) -> vec3<f32> {
    let mu = env_mu(ray_dir, env.dynamic.sun_direction);
    let glow = pow(mu, 32.0);

    return glow * env.dynamic.sun_color * env.dynamic.sun_intensity * 0.4;
}

fn sky_compensation(ray_dir: vec3<f32>) -> f32 {

    let sun_lam = env_half(ray_dir, env.dynamic.sun_direction);
    let sun_inf = pow(sun_lam, 6.0) * env.dynamic.sun_intensity * 0.6;

    let alpha_lam = env_half(ray_dir, env.dynamic.alpha_moon_direction);
    let alpha_inf = pow(alpha_lam, 8.0) * env.dynamic.alpha_moon_intensity * 0.4;

    let beta_lam = env_half(ray_dir, env.dynamic.beta_moon_direction);
    let beta_inf = pow(beta_lam, 8.0) * env.dynamic.beta_moon_intensity * 0.25;

    return 1.0 - (sun_inf + alpha_inf + beta_inf);
}


fn horizon_scattering(ray_dir: vec3<f32>, height: f32) -> vec3<f32> {
    let mu = env_mu(ray_dir, env.dynamic.beta_moon_direction);
    let horizon_glow = pow(1 - height, 4) * pow(mu, 3);

    return horizon_glow * env.dynamic.sun_color * 0.6;
}

fn sky_lights(normal: vec3<f32>) -> vec3<f32> {
    let sun_mu = env_mu(normal, env.dynamic.sun_direction);
    let alpha_mu = env_mu(normal, env.dynamic.alpha_moon_direction);
    let beta_mu = env_mu(normal, env.dynamic.beta_moon_direction);

    let sun_light =
    env.dynamic.sun_color *
    env.dynamic.sun_intensity *
    sun_mu;

    let alpha_light =
    env.dynamic.alpha_moon_color *
    env.dynamic.alpha_moon_intensity *
    alpha_mu * 0.5;

    let beta_light =
    env.dynamic.beta_moon_color *
    env.dynamic.beta_moon_intensity *
    beta_mu * 0.3;

    return sun_light + alpha_light + beta_light;
}

fn env_mu(dir: vec3<f32>, light_dir: vec3<f32>) -> f32 {
    return clamp(dot(dir, normalize(light_dir)), 0.0, 1.0);
}
fn env_half(dir: vec3<f32>, light_dir: vec3<f32>) -> f32 {
    return dot(dir, normalize(light_dir)) * 0.5 + 0.5;
}


fn env_height(ray_dir: vec3<f32>) -> f32 {
    let up = vec3<f32>(0.0, 1.0, 0.0);
    return clamp(dot(ray_dir, up), 0.0, 1.0);
}

fn quantize(color: vec3<f32>) -> vec3<f32> {
    return floor(color * env.constants.quantization) / env.constants.quantization;
}

fn fog(world_pos: vec3<f32>, dist: f32) -> f32 {
    let height_factor = exp(-world_pos.y * env.constants.fog_height_falloff);
    return (1.0 - exp(-dist * env.dynamic.fog_density)) * height_factor;
}
