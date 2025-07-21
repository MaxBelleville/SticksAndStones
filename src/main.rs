
use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;
use bevy_rand::prelude::{ChaCha12Rng, EntropyPlugin};
use noisy_bevy::NoisyShaderPlugin;


mod shared;
mod window_manager; 
mod ui;
mod flycam;
mod compute_system; 
mod contree;
mod world_builder;


fn main() {
    //Initalized the nishita sky model


    App::new()
        .add_plugins((
            DefaultPlugins.set(ImagePlugin::default_nearest()),
            window_manager::WindowManagerPlugin,
            EntropyPlugin::<ChaCha12Rng>::default(),
            NoisyShaderPlugin,
            EnhancedInputPlugin,
            world_builder::WorldBuilderPlugins,

        ))
        .add_systems(Update, close_on_escape)
        .run();
}


fn close_on_escape(keyboard_input: Res<ButtonInput<KeyCode>>, mut writer: EventWriter<AppExit>) {
    if keyboard_input.just_pressed(KeyCode::Escape) {
        writer.write(AppExit::Success);
    }
}
