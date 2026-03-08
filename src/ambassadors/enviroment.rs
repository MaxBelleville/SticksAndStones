use super::TimeManager;
use crate::{renderer, ui};
use bevy::prelude::*;
pub struct EnviromentPlugin;

impl Plugin for EnviromentPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<renderer::EnvironmentUniform>()
            .init_resource::<EnviormentManager>()
            .add_systems(
                Update,
                update_env.run_if(in_state(ui::ManagerState::InGame)),
            );
    }
}

fn update_env(
    // mut sun: Query<&mut Transform, With<SunMarker>>,
    env_mgr: Res<EnviormentManager>,
    time_mgr: Res<TimeManager>,
    mut env: ResMut<renderer::EnvironmentUniform>,
) {
    // let mut tf: Mut<'_, Transform> = sun.single_mut().unwrap();
    // let angle = time_mgr.time_of_day * std::f32::consts::TAU;
    // let rot = Quat::from_rotation_x(angle);
    // tf.rotation = rot;
    let time = time_mgr.get_time();
    let abs_time = time_mgr.get_absolute_time();
    let interp = env_mgr.sample(time, abs_time);
    env.set_interpulated(interp);
}
#[derive(Resource)]
pub struct EnviormentManager {
    sun_intensity: SampleAutoCurve<f32>,
    moon_intensity: SampleAutoCurve<f32>,
    blend: SampleAutoCurve<f32>,
    fog_density: SampleAutoCurve<f32>,
    dawn: renderer::EnviromentPreset,
    noon: renderer::EnviromentPreset,
    dusk: renderer::EnviromentPreset,
    midnight: renderer::EnviromentPreset,
}
impl Default for EnviormentManager {
    fn default() -> Self {
        let interval = interval(0.0, 1.0).unwrap();
        Self {
            sun_intensity: SampleAutoCurve::<f32>::new(interval, Self::sun_intensity()).unwrap(),
            moon_intensity: SampleAutoCurve::<f32>::new(interval, Self::moon_intensity()).unwrap(),
            fog_density: SampleAutoCurve::<f32>::new(interval, Self::fog()).unwrap(),
            blend: SampleAutoCurve::<f32>::new(interval, Self::blend()).unwrap(),
            dawn: renderer::EnviromentPreset::dawn_preset(),
            noon: renderer::EnviromentPreset::noon_preset(),
            dusk: renderer::EnviromentPreset::dusk_preset(),
            midnight: renderer::EnviromentPreset::midnight_preset(),
        }
    }
}
impl EnviormentManager {
    fn sun_intensity() -> [f32; 8] {
        [
            0.3, //Sunrise
            1.0, // noon
            1.0, // noon
            0.3, // sunset
            0.02, 0.0, 0.0, //night
             0.1, //Sunrise
        ]
    }
    fn moon_intensity() -> [f32; 8] {
        [
            0.1, //Sunrise
            0.0, // noon
            0.0, // noon
            0.0, // sunset
            0.3, 1.0, 1.0,//night
             0.3, //Sunrise
        ]
    }
    fn fog() -> [f32; 8] {
        [
            0.0,  // midnight
            0.0, // night
            0.0, 0.0, // morning
            0.0, // noon
            0.0, 0.0, // evening
            0.0,
        ]
    }
    fn blend() -> [f32; 10] {
        [0.0, 0.0,0.1,0.5, 1.0, 1.0, 1.0, 1.0, 0.2, 0.0]
    }

    fn lerp_presets(&self,t:f32)->renderer::EnviromentPreset {
    
        if t < 0.25 {
            let local_t = t / 0.25;
            self.midnight.lerp(&self.dawn, local_t)
        }
        else if t < 0.5 {
            let local_t = (t - 0.25) / 0.25;
            self.dawn.lerp(&self.noon, local_t)
        }
        else if t < 0.75 {
            let local_t = (t - 0.5) / 0.25;
            self.noon.lerp(&self.dusk, local_t)
        }
        else {
            let local_t = (t - 0.75) / 0.25;
            self.dusk.lerp(&self.midnight, local_t)
        }
        
    }

    pub fn sample(&self, t: f32, abs_t: f32) -> renderer::EnviromentInterpolated { 
        let t = t.rem_euclid(1.0);

        let blend = self.blend.sample(t).unwrap();
        let mut interpulated: renderer::EnviromentInterpolated = self.lerp_presets(t).into();

        let sun_intensity = self.sun_intensity.sample(t).unwrap();
        let moon_intensity = self.moon_intensity.sample(t).unwrap();
        let fog_density = self.fog_density.sample(t).unwrap();

        interpulated.set_sun(abs_t, sun_intensity);
        interpulated.set_moons(abs_t, moon_intensity);
        interpulated.set_fog(fog_density);

        interpulated
    }
}
