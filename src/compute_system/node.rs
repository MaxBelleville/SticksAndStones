use bevy::render::render_graph;
use bevy::render::render_resource::{CachedPipelineState, PipelineCacheError};
use bevy::render::renderer::RenderContext;
use bevy::{prelude::*, render::render_resource::PipelineCache};

use super::{ComputeBindGroups, comp_utils, render_pipe,shared,shared::COMPUTE_SHADER_PATH};

enum ComputeState {
    Loading,
    Update(usize),
}

pub struct ComputeRendererNode {
    state: ComputeState,
}

impl Default for ComputeRendererNode {
    fn default() -> Self {
        Self {
            state: ComputeState::Loading,
        }
    }
}

impl render_graph::Node for ComputeRendererNode {
    fn update(&mut self, world: &mut World) {
        let pipeline = world.resource::<render_pipe::ComputeRenderPipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();

        match self.state {
            ComputeState::Loading => {
                match pipeline_cache.get_compute_pipeline_state(pipeline.main_pipeline) {
                    CachedPipelineState::Ok(_) => {
                        self.state = ComputeState::Update(0);
                    }
                    CachedPipelineState::Err(PipelineCacheError::ShaderNotLoaded(_)) => {}
                    CachedPipelineState::Err(err) => {
                        panic!("Loading assets/{COMPUTE_SHADER_PATH}:\n{err}")
                    }
                    _ => {}
                }
            }
            ComputeState::Update(0) => {
                self.state = ComputeState::Update(1);
            }
            ComputeState::Update(1) => {
                self.state = ComputeState::Update(0);
            }
            ComputeState::Update(_) => unreachable!(),
        }
    }

    fn run(
        &self,
        _graph: &mut render_graph::RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), render_graph::NodeRunError> {
         let Some(comp_bind) = world.get_resource::<ComputeBindGroups>() else {
            // Avoid panicking — skip this frame
            return Ok(());
        };

        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = world.resource::<render_pipe::ComputeRenderPipeline>();
        let settings = world.resource::<shared::res::WindowManagerSettings>();

        match self.state {
            ComputeState::Loading => {}
            ComputeState::Update(index) => {
                comp_utils::run_compute_pass(
                    render_context,
                    &comp_bind.output_frames[index],
                    &comp_bind.shared,
                    pipeline_cache,
                    settings.clone(),
                    pipeline.main_pipeline,
                );
            }
        }

        Ok(())
    }
}
