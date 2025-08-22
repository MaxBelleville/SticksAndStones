use bevy::platform::collections::{HashMap};
use bevy::prelude::*;
use bevy::tasks::futures_lite::future;
use bevy::tasks::{block_on, AsyncComputeTaskPool, Task};
use noisy_bevy::{fbm_simplex_3d, fbm_simplex_3d_seeded, simplex_noise_3d};

use super::shared;
use super::chunk;
pub struct ChunkManagerPlugin;


#[derive(Resource,Default)]
struct ChunkTasks {
    data: HashMap<IVec3, Task<chunk::ContreeChunk>>,
}

impl Plugin for ChunkManagerPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ChunkTasks::default())
         .add_systems(
                OnEnter(shared::GameState::InGame),
                init_chunk.run_if(not(resource_exists::<shared::res::IsGamePrepared>)),
            )
        .add_systems(Update, register_chunk);
    }
}

fn init_chunk(mut chunk_tasks: ResMut<ChunkTasks>,mut state: ResMut<shared::res::WorldBuilderResource>,) {
      let task_pool = AsyncComputeTaskPool::get();
      let chunks = collect_chunk_starting();
      let seed = state.noise_seed;
      for (chunk_coord,locals) in chunks {
       
        if chunk_tasks.data.contains_key(&chunk_coord) {
            continue;
        }
        let task = task_pool.spawn(async move {
            let mut chunk =  chunk::ContreeChunk::new(64);
            for local in locals {
                let pos = vec3((local.x+(chunk_coord.x*64)) as f32,(local.y+(chunk_coord.y*64)) as f32,(local.z+(chunk_coord.z*64)) as f32);
                let height = fbm_simplex_3d_seeded(pos * 0.01, 4, 2.0, 0.4,
                    seed*0.002);
                if height>=pos.y*0.04 {
                    chunk.insert(local, Some(1), 0);
                }
            }
            chunk
        });
         chunk_tasks.data.insert(chunk_coord, task);
      }
}

fn register_chunk(mut chunk_tasks: ResMut<ChunkTasks>,
    mut state: ResMut<shared::res::WorldBuilderResource>,
    mut push: ResMut<shared::res::SVTPushConsts>,) {
    let had_some = chunk_tasks.data.len() >0;
    chunk_tasks.data.retain(|chunk_coord, task| {
        // check on our task to see how it's doing :)
        let status = block_on(future::poll_once(task));

        // keep the entry in our HashMap only if the task is not done yet
        let retain = status.is_none();

        // if this task is done, handle the data it returned!
        if let Some(chunk_data) = status {
            state.contree.add_chunk(*chunk_coord,chunk_data);
        }

        retain
    });
    if had_some && chunk_tasks.data.is_empty() {
        state.contree.unify_chunks();
        (state.file_data, state.gpu_data) = state.contree.compress_data();

        push.root_min = (state.contree.get_root_min() + state.contree.get_offset()).as_vec3();
        push.root_size = state.contree.get_root_size() as f32;
    }

}


fn collect_chunk_starting() -> HashMap<IVec3,Vec<IVec3>> {
    let mut chunks = HashMap::new();
    let mut locals = Vec::new();
    for x in 0..64 {
        for y in 0..64 {
            for z in 0..64 {
                locals.push(ivec3(x,y,z));
            }
        }
    }
    
    for x in -2..2 {
        for y in -1..1 {
            for z in -2..2 {
                 chunks.entry(ivec3(x,y,z))
                    .or_insert_with(Vec::new)
                    .append(&mut locals.clone());
            }
        }
    }
    chunks
}