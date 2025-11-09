use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::tasks::futures_lite::future;
use bevy::tasks::{AsyncComputeTaskPool, Task, block_on};
use simdnoise::NoiseBuilder;

use super::chunk;
use super::shared;
pub struct ChunkManagerPlugin;

#[derive(Resource, Default)]
struct ChunkTasks {
    data: HashMap<IVec3, Task<chunk::Chunk>>,
}

impl Plugin for ChunkManagerPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ChunkTasks::default())
            .add_systems(OnEnter(shared::GameState::StartGame), init_chunk)
            .add_systems(Update, (collect_tasks, register_chunk));
    }
}

fn init_chunk(mut state: ResMut<shared::res::WorldData>, time: Res<Time>) {
    for z in -8..8 {
        for y in -1..1 {
            for x in -8..8 {
                state.proccessing_chunks.insert(ivec3(x, y, z));
            }
        }
    }
     info!("Start {:?}", time.elapsed());
}

fn collect_tasks(mut chunk_tasks: ResMut<ChunkTasks>, state: Res<shared::res::WorldData>) {
    let task_pool = AsyncComputeTaskPool::get();

    for chunk_coord in &state.proccessing_chunks {
        let coord = *chunk_coord;
        if chunk_tasks.data.contains_key(chunk_coord) {
            continue;
        }
        if state.contree.has_chunk(coord) {
            continue;
        }
        if !state.contree.chunk_in_bounds(coord) {
            continue;
        }
        let world_coord = coord.as_vec3() * 64.0;
        // Build local coords (64³)

        // Spawn one async task *per chunk*, not per voxel
        let task = task_pool.spawn(async move {
            let mut voxels = Vec::with_capacity(64 * 64);

            let (noise_values, _min, _max) =
            NoiseBuilder::fbm_3d_offset(world_coord.x, 64, world_coord.y, 64, world_coord.z, 64)
                .with_freq(0.05)
                .with_octaves(4)
                .with_gain(2.0)
                .with_seed(1337)
                .with_lacunarity(0.5)
                .generate();

            for z in 0..64 {
                for y in 0..64 {
                    for x in 0..64 {
                        let idx = (z * 64 * 64) + (y * 64) + x;
                        let noise_val = noise_values[idx];

                        let local_voxel = shared::res::Voxel::new(
                            ivec3(x as i32, y as i32, z as i32),
                            1,
                            0,
                        );
                        let world_pos = local_voxel.chunk_to_world(coord).as_vec3a();

                        // Same rule as before
                        if noise_val >= world_pos.y * 0.02 {
                            voxels.push(local_voxel);
                        }
                    }
                }
            }

            let mut chunk = chunk::Chunk::new(voxels.len());
            voxels.sort_unstable_by_key(|v| v.morton);
            chunk.build_region(voxels,0);
           
            chunk
        });

        chunk_tasks.data.insert(coord, task);
    }
   
}

fn register_chunk(
    mut chunk_tasks: ResMut<ChunkTasks>,
    mut game_state: ResMut<NextState<shared::GameState>>,
    mut state: ResMut<shared::res::WorldData>,
    time: Res<Time>
) {
    let had_some = chunk_tasks.data.len() > 0;
    chunk_tasks.data.retain(|chunk_coord, task| {
        // check on our task to see how it's doing :)
        let status = block_on(future::poll_once(task));

        // keep the entry in our HashMap only if the task is not done yet
        let retain = status.is_none();

        // if this task is done, handle the data it returned!
        if let Some(chunk_data) = status {
            state.proccessing_chunks.remove(chunk_coord);
            state.contree.add_chunk(*chunk_coord, chunk_data);
          //  info!("Chunk {:?}", time.elapsed());
        }

        retain
    });

    if had_some && chunk_tasks.data.is_empty() {
         info!("Completed {:?}", time.elapsed());
        state.contree.unify_chunks();
        (state.file_data, state.gpu_data) = state.contree.compress_data();
        game_state.set(shared::GameState::InGame);
        state.root_min = (state.contree.get_root_min() + state.contree.get_offset()).as_vec3();
        state.root_size = state.contree.get_root_size() as f32;
    }
}
