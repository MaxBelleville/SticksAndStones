use bevy::platform::collections::{HashMap};
use bevy::prelude::*;
use bevy::tasks::futures_lite::future;
use bevy::tasks::{block_on, AsyncComputeTaskPool, Task};
use noiz::prelude::*;

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

fn init_chunk(mut chunk_tasks: ResMut<ChunkTasks>,state: ResMut<shared::res::WorldBuilderResource>, world: ResMut<shared::res::AtmosphereUniformData>,) {
      let task_pool = AsyncComputeTaskPool::get();
       let chunk_coords = collect_chunk_coords();

    let mut noise = Noise::<common_noise::Fbm<MixCellGradients<
        OrthoGrid,
        Smoothstep,
        QuickGradients,
    >>>::default();
    noise.seed.re_seed();

    for chunk_coord in chunk_coords {
        if chunk_tasks.data.contains_key(&chunk_coord) {
            continue;
        }
        if !state.contree.chunk_in_bounds(chunk_coord) {
            continue;
        }

        // Spawn one async task *per chunk*, not per voxel
        let noise = noise.clone(); // make it Send
        let task = task_pool.spawn(async move {
           
            let mut voxels = Vec::new();

            // Build local coords (64³)
            for x in 0..64 {
                for y in 0..64 {
                    for z in 0..64 {
                        let local_voxel = shared::res::VoxelVec::new(ivec3(x, y, z), Some(1), 0);
                        let world_pos = local_voxel.chunk_to_world(chunk_coord).as_vec3();
                        let height: f32 = noise.sample(world_pos * 0.01);
                        if height >= world_pos.y * 0.02 {
                            voxels.push(local_voxel);
                        }
                    }
                }
            }
            let mut chunk = chunk::ContreeChunk::new(voxels.len());
            //voxels.sort_unstable_by_key(|v| v.morton);
            chunk.build_all_voxels(voxels);
            chunk
        });

        chunk_tasks.data.insert(chunk_coord, task);
    }
}

fn register_chunk(mut chunk_tasks: ResMut<ChunkTasks>,
    mut state: ResMut<shared::res::WorldBuilderResource>,
    mut push: ResMut<shared::res::SVTPushConsts>, world: ResMut<shared::res::AtmosphereUniformData>,) {
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
         println!("Inserted all {}",  world.time);
        state.contree.unify_chunks();
        (state.file_data, state.gpu_data) = state.contree.compress_data();

        push.root_min = (state.contree.get_root_min() + state.contree.get_offset()).as_vec3();
        push.root_size = state.contree.get_root_size() as f32;
         println!("Done {}",  world.time);
    }
   
}


fn collect_chunk_coords() -> Vec<IVec3> {
    let mut coords = Vec::new();
    for x in -4..4 {
        for y in -1..1 {
            for z in -4..4 {
                coords.push(ivec3(x, y, z));
            }
        }
    }
    coords
}