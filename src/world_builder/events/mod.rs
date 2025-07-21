pub mod handler;
use bevy::prelude::*;

/// Load a chunk from cache or create it if not available.
#[derive(Event)]
pub struct LoadChunkEvent {
    pub key: Vec<u8>,
}

/// Remove chunk from active memory, possibly persisting.
#[derive(Event)]
pub struct UnloadChunkEvent {
    pub key: Vec<u8>,
}

/// Load from disk or serialized memory. May overwrite edits or cache.
#[derive(Event)]
pub struct SerialLoadChunkEvent {
    pub key: Vec<u8>,
}

/// Save chunk to disk or in-memory cache.
#[derive(Event)]
pub struct SaveChunkEvent {
    pub key: Vec<u8>,
}

/// Reload a chunk that was unloaded from cache or generation (preserve edits).
#[derive(Event)]
pub struct ReloadChunkEvent {
    pub key: Vec<u8>,
}

/// Force regeneration of chunk from noise (clear edits/cached).
#[derive(Event)]
pub struct RestartChunkEvent {
    pub key: Vec<u8>,
}

/// Edit a specific voxel.
#[derive(Event)]
pub struct EditVoxelEvent {
    pub position: IVec3,
    pub material: Option<u8>, // None = remove
}