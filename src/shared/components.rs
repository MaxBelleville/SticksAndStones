use bevy::prelude::*;

// Marker for updating the position of the light, not needed unless we have multiple lights
#[derive(Component)]
pub struct Sun;


// Generic system that takes a component as a parameter, and will despawn all entities with that component
fn despawn_screen<T: Component>(to_despawn: Query<Entity, With<T>>, mut commands: Commands) {
    for entity in &to_despawn {
        commands.entity(entity).despawn();
    }
}
