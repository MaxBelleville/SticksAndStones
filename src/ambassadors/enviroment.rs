use std::f32::consts::{PI, TAU};

use super::TimeManager;
use crate::{renderer, ui};
use bevy::prelude::*;
pub struct EnviromentPlugin;

impl Plugin for EnviromentPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<renderer::EnvironmentUniform>()
            .init_resource::<CurveManager>()
            .add_systems(
                Update,
                update_env.run_if(in_state(ui::ManagerState::InGame)),
            );
    }
}

fn update_env(
    // mut sun: Query<&mut Transform, With<SunMarker>>,
    preset_mgr: Res<CurveManager>,
    time_mgr: Res<TimeManager>,
    mut env: ResMut<renderer::EnvironmentUniform>,
) {
    // let mut tf: Mut<'_, Transform> = sun.single_mut().unwrap();
    // let angle = time_mgr.time_of_day * std::f32::consts::TAU;
    // let rot = Quat::from_rotation_x(angle);
    // tf.rotation = rot;
    let tod = time_mgr.get_time_of_day();
    let time = time_mgr.get_time();
    let interp = preset_mgr.sample(time,tod);
    env.set_interpulated(interp);
}
#[derive(Resource)]
pub struct CurveManager {
    sun_intensity: SampleAutoCurve<f32>,
    moon_intensity: SampleAutoCurve<f32>,
    blend: SampleAutoCurve<f32>,
    fog_density: SampleAutoCurve<f32>,
    presets: Vec<renderer::EnviromentPreset>,
}
impl Default for CurveManager {
    fn default() -> Self {
        let interval = interval(0.0, 1.0).unwrap();
        Self {
            sun_intensity: SampleAutoCurve::<f32>::new(interval, CurveManager::sun_intensity())
                .unwrap(),
            moon_intensity: SampleAutoCurve::<f32>::new(interval, CurveManager::moon_intensity())
                .unwrap(),
            fog_density: SampleAutoCurve::<f32>::new(interval, CurveManager::fog()).unwrap(),
            blend: SampleAutoCurve::<f32>::new(interval, CurveManager::blend()).unwrap(),
            presets: vec![
                renderer::EnviromentPreset::dawn_preset(),
                renderer::EnviromentPreset::noon_preset(),
                renderer::EnviromentPreset::dusk_preset(),
                renderer::EnviromentPreset::midnight_preset(),
            ],
        }
    }
}
impl CurveManager {
    fn sun_intensity() -> [f32; 8] {
        [
            0.0, 
            0.0, 
            0.01, 
            0.1,  //sun rise
            0.5, 
            1.0, 
            1.0, //noon
            1.0, 
        ]
    }
    fn moon_intensity() -> [f32; 8] {
        [   
            1.0, 
            1.0, // night
            0.9, 
            0.7, // sunset
            0.2, 
            0.01, 
            0.0, 
            0.0, 
        ]
    }
    fn fog() -> [f32; 8] {
        [
            0.06,
            0.04, 
            0.02,  
            0.03, 
            0.04, 
            0.02, 
            0.01, 
            0.005, 
        ]
    }
    fn blend() -> [f32; 8] {
        [
        0.0,
        0.0,
        0.0,
        0.1,
        0.6,
        1.0,
        1.0,
        1.0
        ]
    }

    pub fn sample(&self, time: f32, tod: f32) -> renderer::EnviromentInterpolated {
        //TODO: Change enviorment to have blend per segement instead of a general blend
        let sun_angle = time * TAU;
        let sun_elevation = sun_angle.sin();

        let sun_point = (sun_elevation + 1.0) * 0.5;

        let segment = tod * self.presets.len() as f32;
        let mut index = segment.floor() as usize;
        if index == self.presets.len() {
            index -= 1;
        }
        let next = (index + 1) % self.presets.len();

        let local_t = segment - index as f32;

        let blend = self.blend.sample(local_t).unwrap();
 
        let preset = self.presets[index].lerp(&self.presets[next], blend);
        let mut interpulated: renderer::EnviromentInterpolated = preset.into();

        let sun_intensity = self.sun_intensity.sample(sun_point).unwrap();
        let moon_intensity = self.moon_intensity.sample(sun_point).unwrap();
        let fog_density = self.fog_density.sample(sun_point).unwrap();

        interpulated.set_sun(sun_angle, sun_intensity);
        interpulated.set_moons(
            sun_angle+PI,
            moon_intensity,
         
        );
        interpulated.set_fog(fog_density);

        interpulated
    }
}

