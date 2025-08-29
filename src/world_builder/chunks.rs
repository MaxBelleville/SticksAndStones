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
                OnEnter(shared::GameState::StartGame),
                init_chunk
            )
        .add_systems(Update, (collect_tasks,register_chunk));
    }
}

fn init_chunk(mut state: ResMut<shared::res::WorldData>) {
    for x in -4..4 {
        for y in -1..1 {
            for z in -4..4 {
                state.proccessing_chunks.insert(ivec3(x, y, z));
            }
        }
    }
    state.noise = Noise::from(common_noise::Fbm::new( 
    Normed::default(),  
    Persistence(0.5),  
    FractalLayers {
        // The layer to do fbm with
        layer: Octave::<MixCellGradients<
            OrthoGrid,
            Smoothstep,
            QuickGradients
        >>::default(),
        // How much to change the scale by between each repetition of the layer
        lacunarity: 2.2,
        // How many repetitions to do
        amount: 3,
    }));
    
    
}

fn collect_tasks(
    mut chunk_tasks: ResMut<ChunkTasks>,
    state: Res<shared::res::WorldData>) {
    let task_pool = AsyncComputeTaskPool::get();
    let noise = state.noise;
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
        
        // Spawn one async task *per chunk*, not per voxel
        let task = task_pool.spawn(async move {
           
            let mut voxels = Vec::with_capacity(64*64);

            // Build local coords (64³)
            for x in 0..64 {
                for y in 0..64 {
                    for z in 0..64 {
                        let local_voxel = shared::res::VoxelVec::new(ivec3(x, y, z), Some(1), 0);
                        let world_pos = local_voxel.chunk_to_world(coord).as_vec3a();
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

        chunk_tasks.data.insert(coord, task);
    }
}


fn register_chunk(
    mut chunk_tasks: ResMut<ChunkTasks>, 
    mut game_state: ResMut<NextState<shared::GameState>>,
    mut state: ResMut<shared::res::WorldData>) {
    let had_some = chunk_tasks.data.len() >0;
    chunk_tasks.data.retain(|chunk_coord, task| {
        // check on our task to see how it's doing :)
        let status = block_on(future::poll_once(task));

        // keep the entry in our HashMap only if the task is not done yet
        let retain = status.is_none();

        // if this task is done, handle the data it returned!
        if let Some(chunk_data) = status {
            state.proccessing_chunks.remove(chunk_coord);
            state.contree.add_chunk(*chunk_coord,chunk_data);
        }

        retain
    });
   
    if had_some && chunk_tasks.data.is_empty() {
        state.contree.unify_chunks();
        (state.file_data, state.gpu_data) = state.contree.compress_data();
        game_state.set(shared::GameState::InGame);
        state.root_min = (state.contree.get_root_min() + state.contree.get_offset()).as_vec3();
        state.root_size = state.contree.get_root_size() as f32;
    }
   
}

