use bevy::{app::PluginGroupBuilder, prelude::*};
/*
What are ambassadors?
1. they bridge the gap between two different systems in the code base.
2. they provide abstract systems that we can use in game (example village/town ambassors)
*/
mod load;
mod world_gen;
mod enviroment;

use crate::ui;
pub struct AmbassorPlugins;
impl PluginGroup for AmbassorPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>().add_group(load::LoadPlugins)
        .add_group(world_gen::WorldGenPlugins)
        .add(GlobalAmbassorPlugin)
        .add(enviroment::EnviromentPlugin)
    }
}
pub struct GlobalAmbassorPlugin;
impl Plugin for GlobalAmbassorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TimeManager>()
        .add_systems(Update, update_time.run_if(in_state(ui::ManagerState::InGame)));
    }
}

pub mod prelude {
    pub use crate::ambassadors::load::LoadState;
    pub use crate::ambassadors::TimeManager;
}
#[derive(Default,Clone,Copy)]
pub enum Season {
    #[default]
    Spring,
    Summer,
    Autumn,
    Winter,
}
impl Season {
    pub fn change_season(&mut self){
        *self = match self {
            Season::Spring => Season::Summer,
            Season::Summer => Season::Autumn,
            Season::Autumn => Season::Winter,
            Season::Winter => Season::Spring,
        };
    }
}

fn update_time(
    mut time_mgr: ResMut<TimeManager>,
    time: Res<Time>,
) {
    time_mgr.time_of_day += time.delta_secs() * time_mgr.day_speed;

    if time_mgr.time_of_day > 1.0 {
        time_mgr.time_of_day -= 1.0;
        time_mgr.day_count += 1;
        if time_mgr.day_count % 48 == 0 {
           time_mgr.season.change_season();
        }
        if time_mgr.day_count % 192 == 0 {
            time_mgr.year_count += 1;
            time_mgr.day_count = 0;
        }
      
    }
}


#[derive(Resource)]
pub struct TimeManager {
    time_of_day: f32,
    day_speed: f32,
    day_count: u32,
    year_count: u32,
    season: Season,
}
impl Default for TimeManager {
    fn default() -> Self {
        Self {
            time_of_day: 0.20,
            day_speed: 0.05,
            day_count: 0,
            year_count: 0,
            season: Season::default(),
        }
    }
}

impl TimeManager {
    pub fn get_day(&self) -> u32 {
        self.day_count
    }
    pub fn get_year(&self) -> u32 {
        self.year_count
    }
    pub fn get_season(&self) -> &Season {
        &self.season
    }
    pub fn get_time_of_day(&self) -> f32 {
        self.time_of_day
    }
    pub fn get_time(&self) -> f32 {
        self.time_of_day + self.day_count as f32
    }
    pub fn get_absolute_time(&self) -> f32 {
        self.time_of_day + self.day_count as f32 + self.year_count as f32
    }
}