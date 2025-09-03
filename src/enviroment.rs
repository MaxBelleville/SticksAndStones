use bevy::asset::RenderAssetUsages;
use bevy::core_pipeline::fxaa::Fxaa;
use bevy::core_pipeline::prepass::MotionVectorPrepass;
use bevy::pbr::CascadeShadowConfigBuilder;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::{
    core_pipeline::prepass::DepthPrepass,
    pbr::light_consts::*,
    prelude::*,
    render::render_resource::TextureUsages,
};
use bevy_atmosphere::prelude::*;
use bevy_enhanced_input::prelude::*;

use bevy_lunex::CameraTextureRenderConstructor;


use super::player;

use super::shared;
pub struct EnviromentPlugin;

// Timer for updating the daylight cycle (updating the atmosphere every frame is slow, so it's better to do incremental changes)
#[derive(Resource)]
struct CycleTimer(Timer);

impl Plugin for EnviromentPlugin {
    fn build(&self, app: &mut App) {
        let atmo_model = AtmosphereModel::new(Nishita {
            sun_position: Vec3::new(0., 0., -1.),
            rayleigh_coefficient: Vec3::new(5.8e-6, 13.5e-6, 22.0e-6),
            mie_scale_height: 1200.0,
            mie_coefficient: 1.7e-5,
            mie_direction: 0.875,
            sun_intensity: 10.0,
            ray_origin: Vec3::new(0.0, 6_371_000.0 + 10.0, 0.0),
            ..default()
        });

        app.add_plugins(AtmospherePlugin).insert_resource(atmo_model)
        .insert_resource(AtmosphereSettings {
            resolution: 256,
            dithering: true,
            ..default()
        })
        .insert_resource(CycleTimer(Timer::new(
            std::time::Duration::from_millis(100),
            TimerMode::Repeating,
        )))
        .insert_resource(shared::res::AtmosphereData {
            sun_dir: Vec3::Y,
            time: 0.0,
        })
        .add_systems(Startup,setup_enviroment)
        .add_systems(
            Update,
            (update_time, daylight_cycle).run_if(in_state(shared::GameState::InGame)),
        );
    }
}
// Simple environment
fn setup_enviroment(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut world: ResMut<shared::res::WorldData>,
    settings: Res<shared::res::WindowManagerSettings>,
     mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
   
    // Our Sun
    commands.spawn((
        DirectionalLight {
            shadows_enabled: true,
            illuminance: lux::RAW_SUNLIGHT,
            ..default()
        },
        CascadeShadowConfigBuilder {
            num_cascades: 3,
            maximum_distance: 10.0,
            ..default()
        }
        .build(),
        shared::components::Sun, // Marks the light as Sun
    ));
    let mut image = Image::new_fill(
        Extent3d {
            width: settings.dim.z.round() as u32,
            height: settings.dim.w.round() as u32,
            ..Default::default()
        },
        TextureDimension::D2,
        &[0, 0, 0, 0],
        TextureFormat::Bgra8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.texture_descriptor.usage = TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::RENDER_ATTACHMENT;

    world.camera= images.add(image.clone());
    world.render= images.add(image);
    let fov_y = 70.0_f32.to_radians(); // Default Bevy Camera3d fov_y is 70 degrees
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(Color::srgb_u8(124, 144, 255))),
        Transform::from_xyz(10.0, 32.0, 0.0),
    ));

    // Simple transform shape just for reference
    //Should draw infront of the compute shader plane
    // Spawn our camera
    commands
        .spawn((
            Camera3d {
                depth_texture_usages: (TextureUsages::RENDER_ATTACHMENT
                    | TextureUsages::TEXTURE_BINDING)
                    .into(),
                ..default()
            },
            Actions::<player::PlayerCam>::default(), // Marks camera as for the flycam plugin
            Projection::from(PerspectiveProjection {
                fov: fov_y,
                ..default()
            }),
            shared::res::ComputeCamera,
            AtmosphereCamera::default(),
            Camera::clear_render_to(world.camera.clone()).with_order(-1),
            Transform::from_xyz(0., 32., 0.).looking_at(vec3(10.0,32.0,0.0),Dir3::Y),
            Msaa::Off,
            DepthPrepass,
            MotionVectorPrepass,
            Fxaa::default(),

        ));
}




fn update_time(time: Res<Time>, mut atmo_data: ResMut<shared::res::AtmosphereData>) {
    atmo_data.time += time.delta_secs();
}

// We can edit the Atmosphere resource and it will be updated automatically
fn daylight_cycle(
    mut atmosphere: AtmosphereMut<Nishita>,
    mut query: Query<(&mut Transform, &mut DirectionalLight), With<shared::components::Sun>>,
    mut timer: ResMut<CycleTimer>,
    mut atmo_data: ResMut<shared::res::AtmosphereData>,
    time: Res<Time>,
) {
    timer.0.tick(time.delta());

    if timer.0.finished() {
        let t = atmo_data.time / 240.0;
        let theta = t % std::f32::consts::TAU;

        let sun_dir = Vec3::new(theta.cos(), theta.sin(), 0.3 * theta.cos()).normalize();
        atmo_data.sun_dir = sun_dir;

        atmosphere.sun_position = sun_dir;
        if let Some(Ok((mut light_trans, mut directional))) = query.single_mut().into() {
            // Light points opposite to sun position
            light_trans.rotation = Quat::from_rotation_arc(Vec3::Y, -sun_dir);

            // Illumination fades in/out near horizon
            directional.illuminance = sun_dir.y.max(0.0).powf(2.0) * lux::RAW_SUNLIGHT;
        }
    }
}


