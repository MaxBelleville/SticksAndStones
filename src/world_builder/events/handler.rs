use bevy::prelude::*;
pub struct EventHandlerPlugin;

impl Plugin for EventHandlerPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<super::LoadChunkEvent>()
        .add_event::<super::UnloadChunkEvent>()
        .add_event::<super::ReloadChunkEvent>()
        .add_event::<super::RestartChunkEvent>()
        .add_event::<super::SaveChunkEvent>()
        .add_event::<super::SerialLoadChunkEvent>()
        .add_event::<super::EditVoxelEvent>();
    }
}


