use bevy::{app::PluginGroupBuilder, prelude::*};
/*
What are ambassadors?
1. they bridge the gap between two different systems in the code base.
2. they provide abstract systems that we can use in game (example village/town ambassors)
*/
mod load;
pub struct AmbassorPlugins;
impl PluginGroup for AmbassorPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>().add_group(load::LoadPlugins)
    }
}
