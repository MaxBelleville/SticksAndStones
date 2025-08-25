use bevy::{
    core_pipeline::prepass::DepthPrepass,
    pbr::{NotShadowCaster, light_consts::*},
    prelude::*,
    render::render_resource::TextureUsages,
};
use bevy_atmosphere::prelude::*;
use bevy_enhanced_input::prelude::*;

use super::compute_system;
use super::flycam;
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

        app.insert_resource(atmo_model)
        .insert_resource(AtmosphereSettings {
            resolution: 256,
            dithering: true,
            ..default()
        })
        .insert_resource(CycleTimer(Timer::new(
            std::time::Duration::from_millis(500),
            TimerMode::Repeating,
        )))
        .insert_resource(shared::res::AtmosphereUniformData {
            sun_dir: Vec3::Y,
            time: 0.0,
        })
        .add_systems(
            OnEnter(shared::GameState::InGame),
            setup_enviroment.run_if(not(resource_exists::<shared::res::IsGamePrepared>)),
        )
        .add_systems(
            Update,
            (update_time, daylight_cycle).run_if(in_state(shared::GameState::InGame)),
        )
        .add_systems(OnEnter(shared::GameState::Paused), pause_blur)
        .add_systems(OnExit(shared::GameState::Paused), ingame_unblur);
    }
}
// Simple environment
fn setup_enviroment(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    settings: Res<shared::res::WindowManagerSettings>,
    mut materials: ResMut<Assets<compute_system::ComputeRenderMaterial>>,
    mut materials2: ResMut<Assets<StandardMaterial>>,
) {
    // Our Sun
    commands.spawn((
        DirectionalLight {
            shadows_enabled: true,
            illuminance: lux::RAW_SUNLIGHT,
            ..default()
        },
        shared::components::Sun, // Marks the light as Sun
    ));

    let dummy_image = Image::default();
    let render_texture_handle = images.add(dummy_image);

    let distance = 3.0; // Distance from camera to plane
    let fov_y = 70.0_f32.to_radians(); // Default Bevy Camera3d fov_y is 70 degrees
    let half_size = shared::maths::screen_size_quad(distance, fov_y, settings.dim);
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
            AtmosphereCamera::default(), // Marks camera as having a skybox, by default it doesn't specify the render layers the skybox can be seen on
            Actions::<flycam::FlyCam>::default(), // Marks camera as for the flycam plugin
            Projection::from(PerspectiveProjection {
                fov: fov_y,
                ..default()
            }),
            Transform::from_xyz(0., 64., 0.),
            Msaa::Off,
            DepthPrepass,
        ))
        .with_children(|parent| {
            //Draw the fullquad plane that will be used for compute shader rendering
            parent.spawn((
                Mesh3d(meshes.add(Plane3d::new(Vec3::new(0., 0., 1.), half_size))),
                // Create a dummy image to use as the render_texture
                MeshMaterial3d(materials.add(compute_system::ComputeRenderMaterial {
                    render_texture: render_texture_handle,
                    is_paused: 0.0
                })),
                Transform::from_xyz(0., 0., -distance),
                compute_system::ComputePlaneComponent,
                NotShadowCaster,
            ));
        });
}


fn update_time(time: Res<Time>, mut atmo_data: ResMut<shared::res::AtmosphereUniformData>) {
    atmo_data.time += time.delta_secs();
}

// We can edit the Atmosphere resource and it will be updated automatically
fn daylight_cycle(
    mut atmosphere: AtmosphereMut<Nishita>,
    mut query: Query<(&mut Transform, &mut DirectionalLight), With<shared::components::Sun>>,
    mut timer: ResMut<CycleTimer>,
    mut atmo_data: ResMut<shared::res::AtmosphereUniformData>,
    time: Res<Time>,
) {
    timer.0.tick(time.delta());

    if timer.0.finished() {
        let t = atmo_data.time / 120.0;
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

fn pause_blur(mut commands: Commands,  mut materials: ResMut<Assets<compute_system::ComputeRenderMaterial>>,
    mesh_query: Query<&MeshMaterial3d<compute_system::ComputeRenderMaterial>, With<compute_system::ComputePlaneComponent>>) {
    commands.insert_resource(AtmosphereSettings {
            resolution: 128,
            dithering: true,
            ..default()
        });
    if let Ok(mesh) = mesh_query.single() {
          if let Some(material) = materials.get_mut(&mesh.0) {
            material.is_paused = 1.0;
          }
    }
}


fn ingame_unblur(mut commands: Commands, mut materials: ResMut<Assets<compute_system::ComputeRenderMaterial>>,
    mesh_query: Query<&MeshMaterial3d<compute_system::ComputeRenderMaterial>, With<compute_system::ComputePlaneComponent>>) {
    commands.insert_resource(AtmosphereSettings {
            resolution: 256,
            dithering: true,
            ..default()
        });
    if let Ok(mesh) = mesh_query.single() {
          if let Some(material) = materials.get_mut(&mesh.0) {
            material.is_paused = 0.0;
          }
    }
}