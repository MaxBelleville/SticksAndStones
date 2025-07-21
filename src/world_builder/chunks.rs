use bevy::prelude::*;
pub struct ChunkManagerPlugin;

impl Plugin for ChunkManagerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init_chunk);
    }
}

fn init_chunk() {

}