
use bevy::{core_pipeline::prepass::DepthPrepass, pbr::{light_consts::*, NotShadowCaster}, prelude::*, render::render_resource::TextureUsages};
use bevy_atmosphere::prelude::*;
use bevy_enhanced_input::prelude::*;

use super::shared;
use super::compute_system;
use super::flycam;
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
            std::time::Duration::from_millis(100),
            TimerMode::Repeating,
        )))
        .insert_resource(shared::res::AtmosphereUniformData {
            sun_dir: Vec3::Y,
            time: 0.0,
        })
        .add_systems(Startup, setup_enviroment).
        add_systems(Update, daylight_cycle);
    }
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
        let t = time.elapsed_secs_wrapped() / 40.0;
        let theta = t % std::f32::consts::TAU;

        let sun_dir = Vec3::new(theta.cos(), theta.sin(), 0.3 * theta.cos()).normalize();
        atmo_data.sun_dir = sun_dir;
        
        atmosphere.sun_position = sun_dir;
        atmo_data.time = t; // or theta if you want looping day time
        if let Some(Ok((mut light_trans, mut directional))) = query.single_mut().into() {
            // Light points opposite to sun position
            light_trans.rotation = Quat::from_rotation_arc(Vec3::Y, -sun_dir);

            // Illumination fades in/out near horizon
            directional.illuminance = sun_dir.y.max(0.0).powf(2.0) * lux::RAW_SUNLIGHT;
        }
    }
}


// Simple environment
fn setup_enviroment(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    settings: Res<shared::res::WindowManagerSettings>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut materials2: ResMut<Assets<compute_system::ComputeRenderMaterial>>,
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

    // Simple transform shape just for reference
    //Should draw infront of the compute shader plane
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::default())),
        MeshMaterial3d(materials.add(StandardMaterial::from(Color::srgb(0.8, 0.0, 0.8)))),
          Transform::from_xyz(0., 0., 0.).with_scale(vec3(0.5,0.5,0.5)),
    ));
    let dummy_image = Image::default();
    let render_texture_handle = images.add(dummy_image);
   
    let distance = 3.0; // Distance from camera to plane
    let fov_y =  70.0_f32.to_radians(); // Default Bevy Camera3d fov_y is 70 degrees
    let half_size = shared::maths::screen_size_quad(distance,fov_y,settings.dim);
    
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
            Transform::from_xyz(0., 10., 0.).looking_at(Vec3::ZERO, Vec3::Y),
            Msaa::Off,
            DepthPrepass
        ))
        .with_children(|parent| {
            //Draw the fullquad plane that will be used for compute shader rendering
            parent.spawn((
                Mesh3d(meshes.add(Plane3d::new(Vec3::new(0.,0.,1.),
                half_size,
                ))),
                // Create a dummy image to use as the render_texture
                MeshMaterial3d(materials2.add(compute_system::ComputeRenderMaterial {
                    render_texture: render_texture_handle,
                })),
                Transform::from_xyz(0., 0., -distance),
                compute_system::ComputePlaneComponent,
                NotShadowCaster,
            ));
        });
}


