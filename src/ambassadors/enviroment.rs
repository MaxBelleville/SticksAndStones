use std::f32::consts::{PI, TAU};

use super::{TimeManager, world};
use crate::{renderer, ui};
use bevy::prelude::*;
use sns_assets::{AssetRef, enviroment};
pub struct EnviromentPlugin;

impl Plugin for EnviromentPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<renderer::EnvironmentUniform>()
            .add_systems(OnEnter(ui::ManagerState::InGame), collect_env)
            .add_systems(
                Update,
                update_env.run_if(in_state(ui::ManagerState::InGame)),
            );
    }
}

pub struct PresetCurve {
    start: enviroment::Preset,
    end: enviroment::Preset,
    curve: SampleAutoCurve<f32>,
    duration: Vec2,
}

#[derive(Resource)]
pub struct CurveManager {
    sun_intensity: SampleAutoCurve<f32>,
    moon_intensity: SampleAutoCurve<f32>,
    fog_density: SampleAutoCurve<f32>,
    preset_curves: Vec<PresetCurve>,
}
impl CurveManager {
    pub fn add_preset(
        &mut self,
        start: enviroment::Preset,
        end: enviroment::Preset,
        curve: enviroment::Curve,
        duration: Vec2,
    ) {
        let interval = interval(0.0, 1.0).unwrap();
        self.preset_curves.push(PresetCurve {
            start,
            end,
            curve: SampleAutoCurve::<f32>::new(interval, curve.values().iter().cloned()).unwrap(),
            duration,
        });
    }
    pub fn new(sun: enviroment::Curve, moon: enviroment::Curve, fog: enviroment::Curve) -> Self {
        let interval = interval(0.0, 1.0).unwrap();
        Self {
            sun_intensity: SampleAutoCurve::<f32>::new(interval, sun.values().iter().cloned())
                .unwrap(),
            moon_intensity: SampleAutoCurve::<f32>::new(interval, moon.values().iter().cloned())
                .unwrap(),
            fog_density: SampleAutoCurve::<f32>::new(interval, fog.values().iter().cloned())
                .unwrap(),
            preset_curves: vec![],
        }
    }
    pub fn sample(&self, time: f32) -> renderer::EnviromentInterpolated {
        let mut t = time;

        let min_start = self
            .preset_curves
            .iter()
            .map(|s| s.duration.x)
            .fold(f32::INFINITY, f32::min);

        if t < min_start {
            t += 1.0;
        }

        let seg = self
            .preset_curves
            .iter()
            .find(|s| t >= s.duration.x && t < s.duration.y + 1e-5)
            .expect("No segment matched time");

        let mut local_t = (t - seg.duration.x) / (seg.duration.y - seg.duration.x);
        local_t = local_t.clamp(0.0, 1.0);

        let blend = seg.curve.sample(local_t).unwrap_or(local_t);

        let preset = seg.start.lerp(&seg.end, blend);
        let mut out: renderer::EnviromentInterpolated = preset.into();

        let sun_angle = t * TAU;
        let sun_height = sun_angle.sin();
        let sun_point = (sun_height + 1.0) * 0.5;

        let sun_intensity = self.sun_intensity.sample(sun_point).unwrap_or(1.0);
        let moon_intensity = self.moon_intensity.sample(sun_point).unwrap_or(0.0);
        let fog_density = self.fog_density.sample(sun_point).unwrap_or(0.02);

        out.set_sun(sun_angle, sun_intensity);
        out.set_moons(sun_angle + PI, moon_intensity);
        out.set_fog(fog_density);

        out
    }
}

fn collect_env(mut commands: Commands, world: Res<world::WorldData>) {
    let registry = world.get_registry();
    let fog_data = registry
        .curve
        .lookup_asset(&AssetRef::from("enviroment:fog".to_string()))
        .unwrap();
    let sun_data = registry
        .curve
        .lookup_asset(&AssetRef::from("enviroment:sun_intensity".to_string()))
        .unwrap();
    let moon_data = registry
        .curve
        .lookup_asset(&AssetRef::from("enviroment:moon_intensity".to_string()))
        .unwrap();

    let mut curve_mgr = CurveManager::new(sun_data.clone(), moon_data.clone(), fog_data.clone());

    for preset in registry.preset_segement.assets() {
        if let Some(p) = preset {
            let start_preset = registry.preset.asset(*p.start_id()).unwrap();
            let end_preset = registry.preset.asset(*p.end_id()).unwrap();
            let blend_data = registry.curve.asset(*p.curve_id()).unwrap();
            let duration = p.duration();
            curve_mgr.add_preset(*start_preset, *end_preset, blend_data.clone(), duration);
        }
    }
    commands.insert_resource(curve_mgr);
}

fn update_env(
    // mut sun: Query<&mut Transform, With<SunMarker>>,
    curve_mgr_opt: Option<Res<CurveManager>>,
    time_mgr: Res<TimeManager>,
    mut env: ResMut<renderer::EnvironmentUniform>,
) {
    let Some(curve_mgr) = curve_mgr_opt else {
        return;
    };

    let time = time_mgr.get_time_of_day();

    let interp = curve_mgr.sample(time);
    env.set_interpulated(interp);
}
